#![feature(unsafe_destructor)]
extern crate libc;
use libc::{c_char, c_int, c_uint, c_void};

use std::os;
use std::ptr;
use std::io::timer::sleep;
use std::c_str::CString;




#[link(name = "dbus-1")]
extern {
    fn dbus_connection_open(address: *const u8,
                            error: *mut DBusError
                           ) -> *mut CDBusConnection;
    fn dbus_connection_open_private(address: *const u8,
                                    error: *mut DBusError
                                   ) -> *mut CDBusConnection;
    fn dbus_connection_unref(connection: *mut CDBusConnection);
    fn dbus_connection_close(connection: *mut CDBusConnection);
    fn dbus_connection_get_server_id(connection: *mut CDBusConnection) -> *const c_char;

    fn dbus_bus_register(connection: *mut CDBusConnection, error: *mut DBusError) -> u32;
    fn dbus_bus_request_name(connection: *mut CDBusConnection, name: *const c_char,
                             flags: c_uint, error: *mut DBusError) -> int;
    fn dbus_error_is_set(error: *const DBusError) -> u32;
    fn dbus_error_init(error: *mut DBusError);
    fn dbus_error_free(error: *mut DBusError);
}


struct DBusError {
    name: *const c_char,
    message: *const c_char,
    _bitfields: c_uint,
    _padding1: *const c_void
}

impl DBusError {
    // Ensure check_safe() is true after getting this back from DBus,
    // if we are in error.
    fn new_unsafe() -> DBusError {
        let mut out = DBusError {
            name: ptr::null(),
            message: ptr::null(),
            _bitfields: 0,
            _padding1: ptr::null()
        };
        unsafe {
            dbus_error_init(&mut out);
        }
        out
    }

    fn check_safe(&self) -> bool {
        self.name.is_not_null() && self.message.is_not_null()
    }

    fn is_set(&self) -> bool {
        unsafe {
            dbus_error_is_set(self) > 0
        }
    }
    
    pub fn get_name(&self) -> CString {
        unsafe {
            CString::new(self.name, false)
        }
    }

    pub fn get_message(&self) -> CString {
        unsafe {
            CString::new(self.message, false)
        }
    }
}

#[unsafe_destructor]
impl Drop for DBusError {
    fn drop(&mut self) {
        unsafe {
            dbus_error_free(self);
        }
    }
}


struct CDBusConnection {
    refcount: i32,
    _extra: [u8, ..1020]
    // ...
}


struct DBusConnection {
    ptr: *mut CDBusConnection
}


#[unsafe_destructor]
impl Drop for DBusConnection {
    fn drop(&mut self) {
        unsafe {
            dbus_connection_close(self.ptr);
            dbus_connection_unref(self.ptr);
        }
    }
}


impl DBusConnection {
    pub fn open(address: &[u8]) -> Result<DBusConnection, DBusError> {
        unsafe {
            let mut error = DBusError::new_unsafe();
            let conn: *mut CDBusConnection = dbus_connection_open_private(
                address.as_ptr(),
                &mut error);

            if error.is_set() {
                assert!(error.check_safe());
                Err(error)
            } else {
                Ok(DBusConnection {
                    ptr: conn
                })
            }
        }
    }

    fn get_server_id(&mut self) -> CString {
        unsafe {
            let buf = dbus_connection_get_server_id(self.ptr);
            CString::new(buf, true)
        }
    }

    fn bus_register(&mut self) -> Result<(), DBusError> {
        let mut error = DBusError::new_unsafe();
        unsafe {
            dbus_bus_register(self.ptr, &mut error);
            if error.is_set() {
                assert!(error.check_safe());
                Err(error)
            } else {
                Ok(())
            }
        }
    }

    fn bus_request_name(&mut self, name: &str, flags: u32) -> Result<int, DBusError> {
        let mut error = DBusError::new_unsafe();
        unsafe {
            let response = name.to_c_str().with_ref(|name_cstr|
                dbus_bus_request_name(self.ptr, name_cstr, flags, &mut error)
            );
            if error.is_set() {
                assert!(error.check_safe());
                Err(error)
            } else {
                Ok(response)
            }
        }
    }
}

fn get_dbus_session_address() -> Option<String> {
    for &(ref key, ref value) in os::env().iter() {
        if key.as_slice() == "DBUS_SESSION_BUS_ADDRESS" {
            return Some(value.clone());
        }
    }
    None
}


#[test]
fn test_connection() {
    let address = match get_dbus_session_address() {
        Some(address) => address,
        None => fail!("Couldn't read environment variable DBUS_SESSION_BUS_ADDRESS")
    };

    let mut dbus_conn = match DBusConnection::open(address.as_bytes()) {
        Ok(conn) => conn,
        Err(err) => {
            println!("DBus Connection failure: {}: {}", err.get_name(), err.get_message());
            return;
        }
    };

    match dbus_conn.bus_register() {
        Ok(_) => (),
        Err(err) => {
            println!("DBus Registration failure failure: {}: {}", err.get_name(), err.get_message());
            return;
        }
    };

    let bus_name = "org.yasashiisyndicate.dbusexample";
    match dbus_conn.bus_request_name(bus_name, 0) {
        Ok(r) => assert!(r > 0),
        Err(err) => {
            println!("DBus RequestName failure failure: {}: {}", err.get_name(), err.get_message());
            return;
        }
    };

    println!("connected to {} as {}", dbus_conn.get_server_id(), bus_name);
}
