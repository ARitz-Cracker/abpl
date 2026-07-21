// Thease are copied from https://www.man7.org/linux/man-pages/man3/sysexits.h.3head.html

/// The command was used incorrectly, e.g., with the wrong number of arguments, a bad flag, bad syntax in a parameter,
/// or whatever.
pub const EX_USAGE: u8 = 64;
/// The input data was incorrect in some way. This should only e used for user's data and not system files.
pub const EX_DATAERR: u8 = 65;
/// An input file (not a system file) did not exist or was not readable.  This could also include errors like "No
/// message" to a mailer (if it cared to catch it).
pub const EX_NOINPUT: u8 = 66;
/// The user specified did not exist. This might be used for mail addresses or remote logins.
pub const EX_NOUSER: u8 = 67;
/// The host specified did not exist. This is used in mail addresses or network requests.
pub const EX_NOHOST: u8 = 68;
/// A service is unavailable. This can occur if a support program or file does not exist. This can also be used as a
/// catch-all message when something you wanted to do doesn't work, but you don't know why.
pub const EX_UNAVAILABLE: u8 = 69;
/// An internal software error has been detected. This should be limited to non-operating system related errors if
/// possible.
pub const EX_SOFTWARE: u8 = 70;
/// An operating system error has been detected. This is intended to be used for such things as "cannot fork", "cannot
/// create pipe", or the like.  It includes things like `getuid(2)` returning a user that does not exist in the
/// `passwd(5)` file.
pub const EX_OSERR: u8 = 71;
/// Some system file (e.g., /etc/passwd, /etc/utmp, etc.)  does not exist, cannot be opened, or has some sort of error
/// (e.g., syntax error).
pub const EX_OSFILE: u8 = 72;
/// A (user specified) output file cannot be created.
pub const EX_CANTCREAT: u8 = 73;
/// An error occurred while doing I/O on some file.
pub const EX_IOERR: u8 = 74;
/// Temporary failure, indicating something that is not really an error. For example that a mailer could not create a
/// connection, and the request should be reattempted later.
pub const EX_TEMPFAIL: u8 = 75;
/// The remote system returned something that was "not possible" during a protocol exchange.
pub const EX_PROTOCOL: u8 = 76;
/// You did not have sufficient permission to perform the operation.  This is not intended for file system problems,
/// which should use EX_NOINPUT or EX_CANTCREAT, but rather for higher level permissions.
pub const EX_NOPERM: u8 = 77;
/// Something was found in an unconfigured or misconfigured state.
pub const EX_CONFIG: u8 = 78;
