use super::panic_win32_error;
use crate::{os::windows::mem::HandleGuard, ExpectWithTracing};
use windows::{
    core::{PCWSTR, PWSTR},
    w,
    Win32::{
        Foundation::{GetLastError, ERROR_INSUFFICIENT_BUFFER, FALSE, HANDLE},
        Security::{
            Authorization::{
                ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
                SDDL_REVISION_1,
            },
            GetTokenInformation, TokenUser, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_USER,
        },
        System::RemoteDesktop::{WTSGetActiveConsoleSessionId, WTSQueryUserToken},
        UI::Shell::wvnsprintfW,
    },
};

pub unsafe fn security_attributes() -> SECURITY_ATTRIBUTES {
    let ptoken = HandleGuard::alloc_zero().expectx("AllocPToken");
    let mut token = HANDLE::from(&ptoken);
    WTSQueryUserToken(WTSGetActiveConsoleSessionId(), &mut token).expectx("WTSQueryUserToken");
    let mut length = 0u32;
    if GetTokenInformation(token, TokenUser, None, 0, &mut length as *mut _) == FALSE
        && GetLastError() != ERROR_INSUFFICIENT_BUFFER
    {
        panic_win32_error("GetTokenInformation");
    }
    let mut token_user = TOKEN_USER::default();
    GetTokenInformation(
        token,
        TokenUser,
        Some(&mut token_user as *mut _ as *mut _),
        length,
        &mut length as *mut _,
    )
    .expectx("GetTokenInformation with user");

    let mut sid = PWSTR::null();
    ConvertSidToStringSidW(token_user.User.Sid, &mut sid as *mut _)
        .expectx("ConvertSidToStringSid");
    let list = [sid];
    let ptr = list.as_ptr();
    let slice = std::ptr::slice_from_raw_parts(ptr, 1) as *const i8;
    let mut sddl = [0u16; 1000];
    wvnsprintfW(
        &mut sddl,
        w!("O:SYG:BAD:(A;;GA;;;SY)(A;;GA;;;%s)S:(ML;;NW;;;ME)"),
        slice,
    );

    let mut lpsp = PSECURITY_DESCRIPTOR::default();
    ConvertStringSecurityDescriptorToSecurityDescriptorW(
        PCWSTR::from_raw(sddl.as_ptr() as *mut _),
        SDDL_REVISION_1,
        &mut lpsp as *mut _,
        None,
    )
    .expectx("ConvertStringSecurityDescriptorToSecurityDescriptor");

    let mut sa = SECURITY_ATTRIBUTES::default();
    sa.bInheritHandle = FALSE;
    sa.lpSecurityDescriptor = lpsp.0;
    sa
}
