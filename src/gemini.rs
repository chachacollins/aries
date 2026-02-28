use std::ffi::CString;
use std::os::raw::c_char;
unsafe extern "C" {
    fn make_request(url: *const c_char) -> *mut c_char;
}

pub fn request(url: &str) -> Result<String, String> {
    let url = CString::new(url).unwrap();
    let res = unsafe { make_request(url.as_ptr()) };
    if res.is_null() {
        Err("request failed".to_string())
    } else {
        let res = unsafe {
            CString::from_raw(res)
                .into_string()
                .expect("invalid utf8 string received")
        };
        Ok(res)
    }
}
