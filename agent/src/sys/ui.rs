use std::process::Command;
use tracing::{debug, error};

pub fn get_password_input() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        get_macos_password()
    }
    #[cfg(target_os = "windows")]
    {
        get_windows_password()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        None
    }
}

#[cfg(target_os = "macos")]
fn get_macos_password() -> Option<String> {
    let script = r#"
        set the_password to ""
        repeat
            display dialog "Enter your SelfControl Passkey (6 digits):" default answer "" with title "SelfControl Setup" with icon caution buttons {"Cancel", "OK"} default button "OK"
            set the_password to text returned of result
            if length of the_password is 6 then
                exit repeat
            else
                display alert "Invalid Passkey" message "Please enter exactly 6 digits." as critical
            end if
        end repeat
        return the_password
    "#;

    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let pwd = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if pwd.is_empty() { None } else { Some(pwd) }
        }
        _ => {
            debug!("MacOS password prompt cancelled or failed");
            None
        }
    }
}

#[cfg(target_os = "windows")]
fn get_windows_password() -> Option<String> {
    // PowerShell script to show a simple input box
    let ps_script = r#"
        Add-Type -AssemblyName Microsoft.VisualBasic;
        $pwd = [Microsoft.VisualBasic.Interaction]::InputBox("Enter your SelfControl Passkey (6 digits):", "SelfControl Setup", "");
        if ($pwd.Length -eq 6) {
            Write-Output $pwd;
        } else if ($pwd.Length -ne 0) {
            [System.Windows.Forms.MessageBox]::Show("Please enter exactly 6 digits.", "Invalid Passkey", 0, 16);
        }
    "#;

    let output = Command::new("powershell")
        .arg("-Command")
        .arg(ps_script)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let pwd = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if pwd.is_empty() { None } else { Some(pwd) }
        }
        _ => {
            debug!("Windows password prompt cancelled or failed");
            None
        }
    }
}
