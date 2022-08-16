// Copyright (C) 2022 by Richard Berry <rjsberry@proton.me>
//
// Permission to use, copy, modify, and/or distribute this software for any
// purpose with or without fee is hereby granted.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
// ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
// OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

//! Smart pointers for applications without allocators.
//!
//! The pointers in this crate are geared at embedded systems. They are
//! _safely_ backed by static data and support dynamic dispatch.
//!
//! # Pointer Types
//!
//! There are two types of pointers: [`Shared`] and [`Unique`].
//!
//! [`Shared`] can be thought of like [`Arc`]; they can be cheaply cloned
//! (without cloning the data behind the pointer) and used by multiple tasks,
//! but the data inside can only be accessed immutably.
//!
//! [`Unique`] can be thought of like [`Box`]; the data inside can be accessed
//! mutably, but cannot be cloned.
//!
//! # Pointer Creation
//!
//! Owned pointers can be created using the [`make_static_shared`] and
//! [`make_static_unique`] macros. These macros return an option — as the
//! macros back the pointers with static data they cannot be called in a
//! loop!
//!
//! # Dynamic Dispatch
//!
//! The type hint in the closure argument to the pointer creation macros ensure
//! the concrete type behind the pointer can have memory allocated with the
//! correct size and alignment. However the smart pointers themselves will
//! happily act as indirection for accessing trait objects! This can be easily
//! achieved by specifying the return argument to the macro:
//!
//! ```
//! use core::any::Any;
//! use qptr::{make_static_unique, Unique};
//!
//! let boxed: Unique<dyn Any> = make_static_unique!(|| -> i32 { 123 }).unwrap();
//! ```
//!
//! # Owned Slices
//!
//! As the pointers can be optionally "fat", they also work with owned slices:
//!
//! ```
//! use qptr::{make_static_unique, Unique};
//!
//! let boxed: Unique<[u8]> = make_static_unique!(|| -> [u8; 3] { [1, 2, 3] }).unwrap();
//! ```
//!
//! [`Shared`]: struct.Shared.html
//! [`Unique`]: struct.Unique.html
//! [`Arc`]: https://doc.rust-lang.org/stable/alloc/sync/struct.Arc.html
//! [`Box`]: https://doc.rust-lang.org/stable/alloc/boxed/struct.Box.html
//! [`make_static_shared`]: macro.make_static_shared.html
//! [`make_static_unique`]: macro.make_static_unique.html

#![no_std]

use core::any::Any;
use core::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use core::fmt::{self, Debug, Display, Formatter};
use core::hash::{Hash, Hasher};
use core::marker::Unpin;
use core::mem;
use core::ops::{Deref, DerefMut};
use core::ptr;

/// A shared owned pointer.
///
/// Create instances with the [`make_static_shared`] macro.
///
/// For more information please consult the crate level documentation.
///
/// [`make_static_shared`]: macro.make_static_shared.html
pub struct Shared<T: ?Sized> {
    ptr: *mut T,
}

/// A unique owned pointer.
///
/// Create instances with the [`make_static_unique`] macro.
///
/// For more information please consult the crate level documentation.
///
/// [`make_static_unique`]: macro.make_static_unique.html
pub struct Unique<T: ?Sized> {
    ptr: *mut T,
}

macro_rules! static_creation {
    ($name:ident, $kind:ty, $kind_str:literal) => {
        static_creation! { $name, $kind, $kind_str, $ }
    };
    ($name:ident, $kind:ty, $kind_str:literal, $d:tt) => {
        #[doc = concat!("Safely creates a ", $kind_str, " pointer using static data")]
        #[doc = ""]
        #[doc = "# Example"]
        #[doc = ""]
        #[doc = "```"]
        #[doc = "use core::any::Any;"]
        #[doc = concat!("use qptr::{", stringify!($name), ", ", stringify!($kind), "};")]
        #[doc = ""]
        #[doc = concat!("let x: ", stringify!($kind), "<dyn Any> = ", stringify!($name), "!(|| -> i32 { 123 }).unwrap();")]
        #[doc = "```"]
        #[macro_export]
        macro_rules! $name {
            (|| -> $d ty:ty { $d ($d arg:tt)+ }) => {{
                use ::core::cell::UnsafeCell;
                use ::core::mem::{self, MaybeUninit};
                use ::core::sync::atomic::{AtomicBool, Ordering};

                #[repr(transparent)]
                struct Obj<T>(UnsafeCell<MaybeUninit<T>>);

                impl<T> Obj<T> {
                    #[inline(always)]
                    pub const fn new() -> Self {
                        Self(UnsafeCell::new(MaybeUninit::uninit()))
                    }

                    #[inline(always)]
                    pub fn get(&self) -> *mut T {
                        unsafe {
                            (&mut *self.0.get()).as_mut_ptr()
                        }
                    }
                }

                unsafe impl<T> Sync for Obj<T> {}

                static OBJ: Obj<$d ty> = Obj::new();
                static OBJ_CLAIMED: AtomicBool = AtomicBool::new(false);

                let buf = OBJ.get() as *mut u8;
                if !OBJ_CLAIMED.swap(true, Ordering::Relaxed) {
                    let mut val: $d ty = { $d($d arg)+ };
                    let val_ptr = &mut val as *mut _;
                    #[allow(unused_unsafe)]
                    let obj = unsafe { $kind::new(buf, &mut val, val_ptr) };
                    mem::forget(val);
                    Some(obj)

                // already claimed from static memory
                } else {
                    None
                }
            }}
        }
    };
}

static_creation!(make_static_shared, Shared, "shared");
static_creation!(make_static_unique, Unique, "unique");

unsafe fn create_obj<T: ?Sized, U>(buf: *mut u8, val: &mut U, mut val_ptr: *mut T) -> *mut T {
    ptr::copy_nonoverlapping(
        val as *const _ as *const u8,
        buf,
        mem::size_of_val::<U>(&val),
    );

    let target = &mut val_ptr as *mut *mut T as *mut *mut u8;
    *target = buf;
    val_ptr
}

// impl Shared

impl<T: ?Sized> Shared<T> {
    #[doc(hidden)]
    pub unsafe fn new<U>(buf: *mut u8, val: &mut U, val_ptr: *mut T) -> Self {
        Self {
            ptr: create_obj(buf, val, val_ptr),
        }
    }
}

impl Shared<dyn Any + 'static> {
    /// Attempts to downcast the shared pointer to a concrete type.
    pub fn downcast<T: Any>(self) -> Result<Shared<T>, Self> {
        if self.is::<T>() {
            Ok(unsafe { self.downcast_unchecked() })
        } else {
            Err(self)
        }
    }
}

impl Shared<dyn Any + Send + 'static> {
    /// Attempts to downcast the shared pointer to a concrete type.
    pub fn downcast<T: Any>(self) -> Result<Shared<T>, Self> {
        if self.is::<T>() {
            Ok(unsafe { self.downcast_unchecked() })
        } else {
            Err(self)
        }
    }
}

impl Shared<dyn Any + Send + Sync + 'static> {
    /// Attempts to downcast the shared pointer to a concrete type.
    pub fn downcast<T: Any>(self) -> Result<Shared<T>, Self> {
        if self.is::<T>() {
            Ok(unsafe { self.downcast_unchecked() })
        } else {
            Err(self)
        }
    }
}

impl<T: ?Sized> Shared<T> {
    unsafe fn downcast_unchecked<U: Any>(self) -> Shared<U> {
        let Self { ptr } = self;
        Shared { ptr: ptr as *mut _ }
    }
}

impl<T: ?Sized> Clone for Shared<T> {
    /// Make a clone of the `Shared` pointer.
    ///
    /// This creates another pointer to the same memory location (the type
    /// behind the pointer does not need to implement `Clone`).
    ///
    /// ```
    /// use qptr::{make_static_shared, Shared};
    ///
    /// let val = make_static_shared!(|| -> i32 { 123 }).unwrap();
    /// let val2 = Shared::clone(&val);
    /// ```
    fn clone(&self) -> Self {
        Self { ptr: self.ptr }
    }
}

impl<T: ?Sized> Deref for Shared<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

unsafe impl<T: Send + Sync + ?Sized> Send for Shared<T> {}
unsafe impl<T: Send + Sync + ?Sized> Sync for Shared<T> {}

impl<T: Debug + ?Sized> Debug for Shared<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl<T: Display + ?Sized> Display for Shared<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&**self, f)
    }
}

impl<T: Eq + ?Sized> Eq for Shared<T> {}

impl<T: ?Sized> fmt::Pointer for Shared<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&(&**self as *const T), f)
    }
}

impl<T: Hash + ?Sized> Hash for Shared<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&**self, state);
    }
}

impl<T: Ord + ?Sized> Ord for Shared<T> {
    fn cmp(&self, other: &Shared<T>) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}

impl<T: PartialEq + ?Sized> PartialEq for Shared<T> {
    fn eq(&self, other: &Shared<T>) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl<T: PartialOrd + ?Sized> PartialOrd for Shared<T> {
    fn partial_cmp(&self, other: &Shared<T>) -> Option<Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
}

impl<T: ?Sized> Unpin for Shared<T> {}

// impl Unique

impl<T: ?Sized> Unique<T> {
    #[doc(hidden)]
    pub unsafe fn new<U>(buf: *mut u8, val: &mut U, val_ptr: *mut T) -> Self {
        Self {
            ptr: create_obj(buf, val, val_ptr),
        }
    }
}

impl Unique<dyn Any + 'static> {
    /// Attempts to downcast the unique pointer to a concrete type.
    pub fn downcast<T: Any>(self) -> Result<Unique<T>, Self> {
        if self.is::<T>() {
            Ok(unsafe { self.downcast_unchecked() })
        } else {
            Err(self)
        }
    }
}

impl Unique<dyn Any + Send + 'static> {
    /// Attempts to downcast the unique pointer to a concrete type.
    pub fn downcast<T: Any>(self) -> Result<Unique<T>, Self> {
        if self.is::<T>() {
            Ok(unsafe { self.downcast_unchecked() })
        } else {
            Err(self)
        }
    }
}

impl Unique<dyn Any + Send + Sync + 'static> {
    /// Attempts to downcast the unique pointer to a concrete type.
    pub fn downcast<T: Any>(self) -> Result<Unique<T>, Self> {
        if self.is::<T>() {
            Ok(unsafe { self.downcast_unchecked() })
        } else {
            Err(self)
        }
    }
}

impl<T: ?Sized> Unique<T> {
    unsafe fn downcast_unchecked<U: Any>(self) -> Unique<U> {
        let Self { ptr } = self;
        Unique { ptr: ptr as *mut _ }
    }
}

impl<T: ?Sized> Deref for Unique<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<T: ?Sized> DerefMut for Unique<T> {
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        unsafe { &mut *self.ptr }
    }
}

unsafe impl<T: Send + ?Sized> Send for Unique<T> {}
unsafe impl<T: Sync + ?Sized> Sync for Unique<T> {}

impl<T: Debug + ?Sized> Debug for Unique<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl<T: Display + ?Sized> Display for Unique<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&**self, f)
    }
}

impl<T: Eq + ?Sized> Eq for Unique<T> {}

impl<T: ?Sized> fmt::Pointer for Unique<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&(&**self as *const T), f)
    }
}

impl<T: Hash + ?Sized> Hash for Unique<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&**self, state);
    }
}

impl<T: Ord + ?Sized> Ord for Unique<T> {
    fn cmp(&self, other: &Unique<T>) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}

impl<T: PartialEq + ?Sized> PartialEq for Unique<T> {
    fn eq(&self, other: &Unique<T>) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl<T: PartialOrd + ?Sized> PartialOrd for Unique<T> {
    fn partial_cmp(&self, other: &Unique<T>) -> Option<Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
}

impl<T: ?Sized> Unpin for Unique<T> {}
