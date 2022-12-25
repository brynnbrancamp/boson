use std::mem;
use std::ptr;
use std::slice;

pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

pub unsafe fn to_bytes<T>(data: &[T]) -> &[u8] {
    slice::from_raw_parts(
        data as *const _ as *const u8,
        mem::size_of::<T>() * data.len(),
    )
}

pub unsafe fn from_bytes<T>(data: &[u8]) -> T {
    if data.len() != mem::size_of::<T>() {
        panic!("byte slice must be same size as type");
    }

    let mut result = mem::MaybeUninit::uninit();

    ptr::copy(
        data as *const _ as *const u8,
        &mut result as *mut _ as *mut u8,
        data.len(),
    );

    result.assume_init()
}

pub unsafe fn ref_bytes<T>(data: &[u8]) -> &T {
    if data.len() != mem::size_of::<T>() {
        panic!("byte slice must be same size as type");
    }

    (data.as_ptr() as *const _ as *const T).as_ref().unwrap()
}

pub unsafe fn mut_bytes<T>(data: &mut [u8]) -> &mut T {
    if data.len() != mem::size_of::<T>() {
        panic!("byte slice must be same size as type");
    }

    (data.as_mut_ptr() as *mut _ as *mut T).as_mut().unwrap()
}
