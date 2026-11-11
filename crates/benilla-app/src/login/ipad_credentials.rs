//! The iPad port's opt-in saved login. The stock client's account name stays in Config.wtf.
use bevy::log::warn;
use security_framework::passwords::{
    delete_generic_password, generic_password, set_generic_password, PasswordOptions,
};

const SERVICE: &str = "org.benilla.wotlk.ipad.login";
const ENTRY: &str = "remembered";
const ITEM_NOT_FOUND: i32 = -25300;

pub(super) fn save(host: &str, account: &str, password: &str) {
    let record = format!("{host}\0{account}\0{password}");
    if let Err(error) = set_generic_password(SERVICE, ENTRY, record.as_bytes()) {
        warn!(
            "login: could not save credentials to Keychain (status {})",
            error.code()
        );
    }
}

pub(super) fn load(host: &str, account: &str) -> Option<String> {
    let data = match generic_password(PasswordOptions::new_generic_password(SERVICE, ENTRY)) {
        Ok(data) => data,
        Err(error) => {
            if error.code() != ITEM_NOT_FOUND {
                warn!("login: could not read Keychain (status {})", error.code());
            }
            return None;
        }
    };
    let record = String::from_utf8(data).ok()?;
    let mut fields = record.splitn(3, '\0');
    let (saved_host, saved_account, password) = (fields.next()?, fields.next()?, fields.next()?);
    (saved_host == host && saved_account.eq_ignore_ascii_case(account) && !password.is_empty())
        .then(|| password.to_owned())
}

pub(super) fn forget() {
    if let Err(error) = delete_generic_password(SERVICE, ENTRY) {
        if error.code() != ITEM_NOT_FOUND {
            warn!("login: could not clear Keychain (status {})", error.code());
        }
    }
}
