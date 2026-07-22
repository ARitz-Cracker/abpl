use std::str::FromStr;

use axum::serve::Listener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::{SocketAddr, SocketAddrParseError, SocketListener};

fn rt() -> tokio::runtime::Runtime {
	tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap()
}

#[test]
fn parse_ip_socket_addr() {
	let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
	assert!(matches!(addr, SocketAddr::Ip(_)));
	assert_eq!(addr.to_string(), "127.0.0.1:8080");
}

#[cfg(unix)]
#[test]
fn parse_unix_socket_addr() {
	let addr: SocketAddr = "unix:///tmp/abpl-test-parse.sock".parse().unwrap();
	assert!(matches!(addr, SocketAddr::Unix(_)));
	assert_eq!(addr.to_string(), "unix:///tmp/abpl-test-parse.sock");
}

#[test]
fn parse_invalid_returns_error() {
	let err = SocketAddr::from_str("not a valid socket address").unwrap_err();
	assert_eq!(err, SocketAddrParseError {});
	assert_eq!(err.to_string(), "string was not a valid unix socket nor a valid ip socket");
}

#[test]
fn eq_and_hash_are_consistent_across_ip_addrs() {
	use std::collections::HashSet;

	let a: SocketAddr = "127.0.0.1:1".parse().unwrap();
	let b: SocketAddr = "127.0.0.1:1".parse().unwrap();
	let c: SocketAddr = "127.0.0.1:2".parse().unwrap();
	assert_eq!(a, b);
	assert_ne!(a, c);

	let mut set = HashSet::new();
	set.insert(a.clone());
	assert!(set.contains(&b), "equal values must hash the same");
	assert!(!set.contains(&c));
}

#[cfg(unix)]
#[test]
fn eq_never_holds_across_unix_and_ip_variants() {
	let ip: SocketAddr = "127.0.0.1:1".parse().unwrap();
	let unix: SocketAddr = "unix:///tmp/abpl-test-crossvariant.sock".parse().unwrap();
	assert_ne!(ip, unix);
}

#[test]
fn serde_round_trips_through_display_and_from_str() {
	let addr: SocketAddr = "127.0.0.1:9999".parse().unwrap();
	let json = serde_json::to_string(&addr).unwrap();
	assert_eq!(json, "\"127.0.0.1:9999\"");
	let round_tripped: SocketAddr = serde_json::from_str(&json).unwrap();
	assert_eq!(round_tripped, addr);
}

#[cfg(all(feature = "app", feature = "std"))]
#[test]
fn socket_addr_parse_error_provides_exit_code() {
	use crate::providers::ProvidesExitCode;
	assert_eq!(
		SocketAddrParseError {}.exit_code(),
		crate::app::consts::EX_CONFIG.into()
	);
}

#[test]
fn tcp_listener_binds_accepts_and_round_trips_bytes() {
	rt().block_on(async {
		let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
		let mut listener = SocketListener::bind(addr).await.unwrap();
		let bound = Listener::local_addr(&listener).unwrap();
		let SocketAddr::Ip(bound_ip) = bound else {
			panic!("a TCP bind should resolve to an `Ip` addr")
		};

		let client_task = tokio::spawn(async move {
			let mut stream = tokio::net::TcpStream::connect(bound_ip).await.unwrap();
			stream.write_all(b"ping").await.unwrap();
			let mut buf = [0u8; 4];
			stream.read_exact(&mut buf).await.unwrap();
			assert_eq!(&buf, b"pong");
		});

		let (mut server_stream, _addr) = listener.accept().await;
		let mut buf = [0u8; 4];
		server_stream.read_exact(&mut buf).await.unwrap();
		assert_eq!(&buf, b"ping");
		server_stream.write_all(b"pong").await.unwrap();

		client_task.await.unwrap();
	});
}

#[cfg(unix)]
#[test]
fn unix_listener_binds_accepts_and_round_trips_bytes() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("abpl-test.sock");
	rt().block_on(async {
		let addr: SocketAddr = format!("unix://{}", path.display()).parse().unwrap();
		let mut listener = SocketListener::bind(addr.clone()).await.unwrap();
		assert_eq!(listener.original_addr(), addr);

		let connect_path = path.clone();
		let client_task = tokio::spawn(async move {
			let mut stream = tokio::net::UnixStream::connect(&connect_path).await.unwrap();
			stream.write_all(b"ping").await.unwrap();
			let mut buf = [0u8; 4];
			stream.read_exact(&mut buf).await.unwrap();
			assert_eq!(&buf, b"pong");
		});

		let (mut server_stream, _addr) = listener.accept().await;
		let mut buf = [0u8; 4];
		server_stream.read_exact(&mut buf).await.unwrap();
		assert_eq!(&buf, b"ping");
		server_stream.write_all(b"pong").await.unwrap();

		client_task.await.unwrap();
	});
}
