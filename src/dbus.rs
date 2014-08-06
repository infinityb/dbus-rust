#![feature(unsafe_destructor)]
#![allow(dead_code)]

extern crate libc;
use libc::{c_char, c_int, c_uint, c_void};

use std::os;
use std::ptr;
use std::io::timer::sleep;
use std::c_str::CString;


type DBusResult<T> = Result<T, DBusError>;


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
    fn dbus_connection_dispatch(connection: *mut CDBusConnection) -> c_int;

    fn dbus_bus_register(connection: *mut CDBusConnection, error: *mut DBusError) -> u32;
    fn dbus_bus_request_name(connection: *mut CDBusConnection, name: *const c_char,
                             flags: c_uint, error: *mut DBusError) -> c_int;
    fn dbus_error_is_set(error: *const DBusError) -> u32;
    fn dbus_error_init(error: *mut DBusError);
    fn dbus_error_free(error: *mut DBusError);
}


pub type DBusInterfaceElement = self::DBusInterfaceElement::DBusInterfaceElement;
pub mod DBusInterfaceElement {
    pub enum DBusInterfaceElement {
        Method(String, String, Vec<String>, String),
        Signal(String, String)
    }
}


struct DBusInterface {
    name: String,
    members: Vec<DBusInterfaceElement>
}

impl DBusInterface {
    pub fn new(name: &str) -> DBusInterface {
        DBusInterface {
            name: String::from_str(name),
            members: Vec::new()
        }
    }

    #[inline]
    pub fn add_member(&mut self, elem: DBusInterfaceElement) {
        self.members.push(elem);
    }

    pub fn add_method(&mut self, name: &str, argspec: &str,
                      argnames: Vec<String>, retspec: &str) {
        self.add_member(DBusInterfaceElement::Method(
            String::from_str(name),
            String::from_str(argspec),
            argnames,
            String::from_str(retspec)
        ));
    }
    
    pub fn add_signal(&mut self, name: &str, retspec: &str) {
        self.add_member(DBusInterfaceElement::Signal(
            String::from_str(name),
            String::from_str(retspec)
        ));
    }
}


pub type DBusDispatchStatus = self::DBusDispatchStatus::DBusDispatchStatus;
pub mod DBusDispatchStatus {
    // from dbus.h
    pub static DATA_REMAINS: i32 = 0;
    pub static COMPLETE: i32 = 1;
    pub static NEED_MEMORY: i32 = 2;

    pub enum DBusDispatchStatus {
        DataRemains,
        Complete,
        NeedMemory,
        Unknown(i32)
    }

    // _ => fail!("Unsupported DBusDispatchStatus: {}", result)
    pub fn from_ord(result: i32) -> DBusDispatchStatus {
        match result {
            DATA_REMAINS => DataRemains,
            COMPLETE => Complete,
            NEED_MEMORY => NeedMemory,
            _ => Unknown(result)
            
        }
    }
}


pub type DBusHandlerResult = self::DBusHandlerResult::DBusHandlerResult;
pub mod DBusHandlerResult {
    // from dbus.h
    pub static HANDLED: i32 = 0;
    pub static NOT_YET_HANDLED: i32 = 1;
    pub static NEED_MEMORY: i32 = 2;

    pub enum DBusHandlerResult {
        Handled,
        NotYetHandled,
        NeedMemory,
        Unknown(i32)
    }

    #[inline]
    pub fn from_ord(result: i32) -> DBusHandlerResult {
        match result {
            HANDLED => Handled,
            NOT_YET_HANDLED => NotYetHandled,
            NEED_MEMORY => NeedMemory,
            _ => Unknown(result)
        }
    }
}


pub type DBusTimeout = self::DBusTimeout::DBusTimeout;
pub mod DBusTimeout {
    pub enum DBusTimeout {
        Default,
        Infinite,
        Milliseconds(i32)
    }

    #[inline]
    pub fn default() -> DBusTimeout {
        Default
    }

    #[inline]
    pub fn infinite() -> DBusTimeout {
        Infinite
    }

    #[inline]
    pub fn millis(millis: i32) -> DBusTimeout {
        if 0 <= millis && millis < 0x7FFFFFFF {
            Milliseconds(millis)
        } else {
            fail!("0 <= millis < 0x7FFFFFFF");
        }
    }
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
    pub fn open(address: &[u8]) -> DBusResult<DBusConnection> {
        let mut error = DBusError::new_unsafe();
        let conn: *mut CDBusConnection = unsafe {
            dbus_connection_open_private(
                address.as_ptr(),
                &mut error)
        };
        if error.is_set() {
            assert!(error.check_safe());
            Err(error)
        } else {
            Ok(DBusConnection {
                ptr: conn
            })
        }
    }

    pub fn get_server_id(&mut self) -> CString {
        unsafe {
            let buf = dbus_connection_get_server_id(self.ptr);
            CString::new(buf, true)
        }
    }

    pub fn bus_register(&mut self) -> Result<(), DBusError> {
        let mut error = DBusError::new_unsafe();
        unsafe {
            dbus_bus_register(self.ptr, &mut error);
        }
        if error.is_set() {
            assert!(error.check_safe());
            Err(error)
        } else {
            Ok(())
        }
    }

    pub fn bus_request_name(&mut self, name: &str, flags: u32) -> DBusResult<i32> {
        let mut error = DBusError::new_unsafe();
        let name_cstr = name.to_c_str();
        let response = unsafe {
            dbus_bus_request_name(self.ptr, name_cstr.as_ptr(), flags, &mut error)
        };
        if error.is_set() {
            if error.check_safe() {
                fail!("unsafe error after dbus_bus_request_name");
            }
            Err(error)
        } else {
            assert!(response > 0);
            Ok(response)
        }
    }

    pub fn dispatch(&mut self) -> DBusDispatchStatus {
        DBusDispatchStatus::from_ord(unsafe {
            dbus_connection_dispatch(self.ptr)
        })
    }
}

pub fn get_dbus_session_address() -> Option<String> {
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
            fail!("DBus Connection failure: {}: {}", err.get_name(), err.get_message());
        }
    };

    match dbus_conn.bus_register() {
        Ok(_) => (),
        Err(err) => {
            fail!("DBus Registration failure failure: {}: {}", err.get_name(), err.get_message());
        }
    };

    let bus_name = "org.yasashiisyndicate.dbusexample";
    match dbus_conn.bus_request_name(bus_name, 0) {
        Ok(_) => (),
        Err(err) => {
            fail!("DBus RequestName failure failure: {}: {}", err.get_name(), err.get_message());
        }
    };

    println!("connected to {} as {}", dbus_conn.get_server_id(), bus_name);
}


#[test]
fn test_dbus_interface() {
    let mut dbus_introspectable = DBusInterface::new(
        "org.freedesktop.DBus.Introspectable");
    dbus_introspectable.add_method("Introspect", "", vec![], "");

    let mut dbus_peer = DBusInterface::new("org.freedesktop.DBus.Peer");
    dbus_peer.add_method("Ping", "", vec![], "");
    dbus_peer.add_method("GetMachineId", "", vec![], "s");

    let mut frobulator = DBusInterface::new("org.yasashiisyndicate.Frobulator");
    frobulator.add_method("Frobulate", "s", vec![String::from_str("value")], "s");
}


pub static frobulator: DBusInterface = {
    let mut frobulator = DBusInterface::new("org.yasashiisyndicate.Frobulator");
    frobulator.add_method("Frobulate", "s", vec![String::from_str("value")], "s");
    frobulator
};