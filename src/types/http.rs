use std::{
	convert::Infallible,
	error::Error,
	fmt::Display,
	hash::Hash,
	io::Result as IoResult,
	net::SocketAddr as NetSocketAddr,
	pin::Pin,
	str::FromStr,
	task::{Context as AsyncContext, Poll},
};
#[cfg(unix)]
use std::{
	fs::Permissions,
	os::unix::{fs::PermissionsExt as _, net::SocketAddr as UnixSocketAddr},
	path::Path,
};

use axum::{
	extract::Request,
	response::Response,
	serve::{Listener, Serve},
};
use serde_with::{DeserializeFromStr, SerializeDisplay};

#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};
use tokio::{
	io::{AsyncRead, AsyncWrite, ReadBuf},
	net::{TcpListener, TcpStream},
};
use tower_service::Service;

use crate::providers::ProvidesExitCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketAddrParseError {}
impl Display for SocketAddrParseError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("string was not a valid unix socket nor a valid ip socket")
	}
}
impl Error for SocketAddrParseError {}
#[cfg(feature = "app")]
impl ProvidesExitCode for SocketAddrParseError {
	fn exit_code(&self) -> std::process::ExitCode {
		crate::app::consts::EX_CONFIG.into()
	}
}

#[derive(Debug, Clone, DeserializeFromStr, SerializeDisplay)]
pub enum SocketAddr {
	#[cfg(unix)]
	Unix(UnixSocketAddr),
	Ip(NetSocketAddr),
	// What other socket types would there be?
	#[non_exhaustive]
	Unknown,
}
impl Display for SocketAddr {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			#[cfg(unix)]
			Self::Unix(socket_addr) => {
				f.write_str("unix://")?;
				f.write_str(
					&socket_addr
						.as_pathname()
						.map(|sa| sa.to_string_lossy())
						.unwrap_or_default(),
				)
			},
			Self::Ip(socket_addr) => socket_addr.fmt(f),
			_ => unimplemented!(),
		}
	}
}
impl PartialEq for SocketAddr {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			#[cfg(unix)]
			(Self::Unix(l0), Self::Unix(r0)) => {
				// We're assuming that unnamed sockets won't be created
				l0.as_pathname() == r0.as_pathname()
			},
			(Self::Ip(l0), Self::Ip(r0)) => l0 == r0,
			_ => std::mem::discriminant(self) == std::mem::discriminant(other),
		}
	}
}
impl Eq for SocketAddr {}
impl Hash for SocketAddr {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		std::mem::discriminant(self).hash(state);
		match self {
			#[cfg(unix)]
			SocketAddr::Unix(socket_addr) => socket_addr.as_pathname().hash(state),
			SocketAddr::Ip(socket_addr) => socket_addr.hash(state),
			_ => {},
		}
	}
}

impl FromStr for SocketAddr {
	type Err = SocketAddrParseError;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		#[cfg(unix)]
		if let Some(path_str) = s.strip_prefix("unix://") {
			let path = Path::new(&path_str);
			return Ok(Self::Unix(
				UnixSocketAddr::from_pathname(path).map_err(|_| SocketAddrParseError {})?,
			));
		}

		Ok(Self::Ip(
			NetSocketAddr::from_str(s).map_err(|_| SocketAddrParseError {})?,
		))
	}
}

#[derive(Debug)]
pub enum SocketStream {
	Tcp(TcpStream),
	#[cfg(unix)]
	Unix(UnixStream),
	#[non_exhaustive]
	Unknown,
}
impl AsyncRead for SocketStream {
	fn poll_read(mut self: Pin<&mut Self>, cx: &mut AsyncContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<IoResult<()>> {
		match &mut *self {
			Self::Tcp(tcp_stream) => Pin::new(tcp_stream).poll_read(cx, buf),
			#[cfg(unix)]
			Self::Unix(unix_stream) => Pin::new(unix_stream).poll_read(cx, buf),
			Self::Unknown => unreachable!(),
		}
	}
}
impl AsyncWrite for SocketStream {
	fn poll_write(mut self: Pin<&mut Self>, cx: &mut AsyncContext<'_>, buf: &[u8]) -> Poll<IoResult<usize>> {
		match &mut *self {
			Self::Tcp(tcp_stream) => Pin::new(tcp_stream).poll_write(cx, buf),
			#[cfg(unix)]
			Self::Unix(unix_stream) => Pin::new(unix_stream).poll_write(cx, buf),
			Self::Unknown => unreachable!(),
		}
	}
	fn poll_flush(mut self: Pin<&mut Self>, cx: &mut AsyncContext<'_>) -> Poll<IoResult<()>> {
		match &mut *self {
			Self::Tcp(tcp_stream) => Pin::new(tcp_stream).poll_flush(cx),
			#[cfg(unix)]
			Self::Unix(unix_stream) => Pin::new(unix_stream).poll_flush(cx),
			Self::Unknown => unreachable!(),
		}
	}
	fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut AsyncContext<'_>) -> Poll<IoResult<()>> {
		match &mut *self {
			Self::Tcp(tcp_stream) => Pin::new(tcp_stream).poll_shutdown(cx),
			#[cfg(unix)]
			Self::Unix(unix_stream) => Pin::new(unix_stream).poll_shutdown(cx),
			Self::Unknown => unreachable!(),
		}
	}
}

#[derive(Debug)]
pub enum SocketListener {
	Tcp(TcpListener, NetSocketAddr),
	#[cfg(unix)]
	Unix(UnixListener, UnixSocketAddr),
	#[non_exhaustive]
	Unknown,
}

impl SocketListener {
	pub async fn bind(socket: SocketAddr) -> IoResult<Self> {
		match socket {
			#[cfg(unix)]
			SocketAddr::Unix(addr) => {
				let tokio_addr = addr.clone().into();
				let listener = UnixListener::bind_addr(&tokio_addr)?;
				if let Some(path) = addr.as_pathname() {
					// This mode will mostly be used to hook up to reverse-proxies. If for any reason this is being
					// used as a "trusted" IPC, at the very least, the socket should be in a private folder.
					// Ignoring the error since this is "best-effort". Blocking should be fine since sockets should be
					// in tempfs anyway :^)
					if let Err(err) = std::fs::set_permissions(path, Permissions::from_mode(0o777)) {
						let path_str = path.to_string_lossy();
						tracing::warn!("set_permissions for {path_str}: {err}");
					}
				}
				Ok(Self::Unix(listener, addr))
			},
			SocketAddr::Ip(addr) => Ok(Self::Tcp(TcpListener::bind(addr).await?, addr)),
			_ => unimplemented!(),
		}
	}

	pub fn serve<
		M: for<'a> Service<axum::serve::IncomingStream<'a, Self>, Error = Infallible, Response = S>,
		S: Service<Request, Response = Response, Error = Infallible> + Clone + Send + 'static,
	>(
		self,
		make_service: M,
	) -> Serve<Self, M, S>
	where
		S::Future: Send,
	{
		axum::serve(self, make_service)
	}

	/// Returns the `SocketAddr` originally specified in `bind`. This may differ from `local_addr`, as this does not
	/// return the resolved address, e.g., a TCP socket being started on a ephemeral port.
	pub fn original_addr(&self) -> SocketAddr {
		match self {
			Self::Tcp(_, addr) => SocketAddr::Ip(*addr),
			#[cfg(unix)]
			Self::Unix(_, addr) => SocketAddr::Unix(addr.clone()),
			Self::Unknown => unreachable!(),
		}
	}
}

impl Listener for SocketListener {
	type Io = SocketStream;
	type Addr = SocketAddr;
	async fn accept(&mut self) -> (Self::Io, Self::Addr) {
		match self {
			SocketListener::Tcp(inner, _) => {
				let (io, addr) = Listener::accept(inner).await;
				(SocketStream::Tcp(io), SocketAddr::Ip(addr))
			},
			#[cfg(unix)]
			SocketListener::Unix(inner, _) => {
				let (io, addr) = Listener::accept(inner).await;
				(SocketStream::Unix(io), SocketAddr::Unix(addr.into()))
			},
			SocketListener::Unknown => unreachable!(),
		}
	}
	fn local_addr(&self) -> tokio::io::Result<Self::Addr> {
		match self {
			SocketListener::Tcp(inner, _) => Listener::local_addr(inner).map(SocketAddr::Ip),
			#[cfg(unix)]
			SocketListener::Unix(inner, _) => Listener::local_addr(inner).map(|addr| SocketAddr::Unix(addr.into())),
			SocketListener::Unknown => unreachable!(),
		}
	}
}

#[cfg(test)]
#[path = "../tests/types/http.rs"]
mod tests;
