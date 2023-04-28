use std::mem::transmute;

use anyhow::{anyhow, bail, Result};
use objc::runtime::{Class, Object};
use objc_foundation::{INSArray, INSObject, INSString, NSArray, NSDictionary, NSObject, NSString};
use objc_id::Id;

pub struct OSXClipboard {
    pasteboard: Id<Object>,
}

// required to bring NSPasteboard into the path of the class-resolver
#[link(name = "AppKit", kind = "framework")]
extern "C" {}

// https://github.com/aweinstock314/rust-clipboard/blob/master/src/osx_clipboard.rs
impl OSXClipboard {
    pub fn new() -> Result<Self> {
        let cls = Class::get("NSPasteboard").ok_or(anyhow!("Failed to get NSPasteboard class"))?;
        let pasteboard: *mut Object = unsafe { msg_send![cls, generalPasteboard] };
        if pasteboard.is_null() {
            bail!("Failed to generalPasteboard")
        }
        let pasteboard = unsafe { Id::from_ptr(pasteboard) };
        Ok(Self { pasteboard })
    }

    pub fn get_text(&self) -> Result<Option<String>> {
        let string_class: Id<NSObject> = {
            let cls: Id<Class> = unsafe { Id::from_ptr(class("NSString")) };
            unsafe { transmute(cls) }
        };
        let classes = NSArray::from_vec(vec![string_class]);
        let options: Id<NSDictionary<NSObject, NSObject>> = NSDictionary::new();
        let string_array: Id<NSArray<NSString>> = unsafe {
            let obj: *mut NSArray<NSString> =
                msg_send![self.pasteboard, readObjectsForClasses: &*classes options: &*options];
            if obj.is_null() {
                bail!("Faild to readObjectsForClasses")
            }
            Id::from_ptr(obj)
        };
        if string_array.count() == 0 {
            return Ok(None);
        }
        Ok(Some(string_array[0].as_str().to_owned()))
    }

    pub fn set_text(&self, data: String) -> Result<()> {
        let string_array = NSArray::from_vec(vec![NSString::from_str(&data)]);
        let _: usize = unsafe { msg_send![self.pasteboard, clearContents] };
        let success: bool = unsafe { msg_send![self.pasteboard, writeObjects: string_array] };
        if !success {
            bail!("Failed to set clipboard text")
        }
        Ok(())
    }
}

// this is a convenience function that both cocoa-rs and
//  glutin define, which seems to depend on the fact that
//  Option::None has the same representation as a null pointer
#[inline]
fn class(name: &str) -> *mut Class {
    unsafe { transmute(Class::get(name)) }
}
