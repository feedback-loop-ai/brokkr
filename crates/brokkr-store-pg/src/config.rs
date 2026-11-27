//! PostgreSQL connection configuration parsed from `postgres://` URLs.

use std::time::Duration;

use crate::PgError;

/// Default PostgreSQL TCP port.
pub const DEFAULT_PORT: u16 = 5432;
/// Default TCP connect timeout.
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Default TCP read/write timeout.
pub const DEFAULT_IO_TIMEOUT: Duration = Duration::from_secs(30);

/// Connection settings for [`crate::PgStore`] and its port adapters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: Option<String>,
    pub database: String,
    pub connect_timeout: Duration,
    pub io_timeout: Duration,
}

impl PgConfig {
    /// Parse a `postgres://user[:password]@host[:port]/database` (or
    /// `postgresql://...`) connection URL, refusing missing components,
    /// invalid percent-escapes, NUL bytes, or unknown query parameters.
    pub fn parse(url: &str) -> Result<Self, PgError> {
        let rest = url
            .strip_prefix("postgres://")
            .or_else(|| url.strip_prefix("postgresql://"))
            .ok_or(PgError::InvalidUrl(
                "scheme must be postgres:// or postgresql://",
            ))?;
        let (path_part, query) = match rest.split_once('?') {
            Some((left, right)) => (left, Some(right)),
            None => (rest, None),
        };
        let (authority, raw_db) = path_part
            .split_once('/')
            .ok_or(PgError::InvalidUrl("missing /database path"))?;
        if raw_db.is_empty() || raw_db.contains('/') {
            return Err(PgError::InvalidUrl(
                "database name must be a single segment",
            ));
        }
        let database = decode_cstr_component(raw_db)?;
        let (userinfo, hostport) = authority
            .rsplit_once('@')
            .ok_or(PgError::InvalidUrl("missing user@host authority"))?;
        let (user, password) = parse_userinfo(userinfo)?;
        let (host, port) = parse_host_port(hostport)?;
        let (connect_timeout, io_timeout) = parse_query_timeouts(query)?;
        Ok(Self {
            host,
            port,
            user,
            password,
            database,
            connect_timeout,
            io_timeout,
        })
    }

    /// Override the TCP connect timeout.
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Override the TCP read/write timeout.
    pub fn with_io_timeout(mut self, timeout: Duration) -> Self {
        self.io_timeout = timeout;
        self
    }

    pub(crate) fn host_port(&self) -> (&str, u16) {
        (&self.host, self.port)
    }
}

fn parse_userinfo(userinfo: &str) -> Result<(String, Option<String>), PgError> {
    let (raw_user, raw_pass) = match userinfo.split_once(':') {
        Some((u, p)) => (u, Some(p)),
        None => (userinfo, None),
    };
    if raw_user.is_empty() {
        return Err(PgError::InvalidUrl("user must not be empty"));
    }
    let user = decode_cstr_component(raw_user)?;
    let password = raw_pass.map(decode_cstr_component).transpose()?;
    Ok((user, password))
}

fn parse_host_port(hostport: &str) -> Result<(String, u16), PgError> {
    if let Some(after_open) = hostport.strip_prefix('[') {
        let (host, rest) = after_open
            .split_once(']')
            .ok_or(PgError::InvalidUrl("unclosed IPv6 host bracket"))?;
        if host.is_empty() {
            return Err(PgError::InvalidUrl("host must not be empty"));
        }
        let port = match rest {
            "" => DEFAULT_PORT,
            _ => parse_port(
                rest.strip_prefix(':')
                    .ok_or(PgError::InvalidUrl("expected :port after IPv6 host"))?,
            )?,
        };
        return Ok((host.to_string(), port));
    }
    let (host, port) = match hostport.rsplit_once(':') {
        Some((h, p)) => (h, parse_port(p)?),
        None => (hostport, DEFAULT_PORT),
    };
    if host.is_empty() {
        return Err(PgError::InvalidUrl("host must not be empty"));
    }
    Ok((host.to_string(), port))
}

fn parse_port(raw: &str) -> Result<u16, PgError> {
    let port: u16 = raw
        .parse()
        .map_err(|_| PgError::InvalidUrl("invalid port number"))?;
    if port == 0 {
        return Err(PgError::InvalidUrl("port must be non-zero"));
    }
    Ok(port)
}

fn parse_query_timeouts(query: Option<&str>) -> Result<(Duration, Duration), PgError> {
    let mut connect = DEFAULT_CONNECT_TIMEOUT;
    let mut io = DEFAULT_IO_TIMEOUT;
    let Some(q) = query.filter(|s| !s.is_empty()) else {
        return Ok((connect, io));
    };
    for pair in q.split('&') {
        let (key, val) = pair
            .split_once('=')
            .ok_or(PgError::InvalidUrl("malformed query parameter"))?;
        let secs: u64 = val
            .parse()
            .map_err(|_| PgError::InvalidUrl("timeout parameter must be seconds"))?;
        if secs == 0 {
            return Err(PgError::InvalidUrl("timeout must be positive"));
        }
        match key {
            "connect_timeout" => connect = Duration::from_secs(secs),
            "io_timeout" => io = Duration::from_secs(secs),
            _ => return Err(PgError::InvalidUrl("unsupported query parameter")),
        }
    }
    Ok((connect, io))
}

fn decode_cstr_component(raw: &str) -> Result<String, PgError> {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hi = hex_nibble(bytes.get(i + 1).copied())?;
            let lo = hex_nibble(bytes.get(i + 2).copied())?;
            out.push((hi << 4) | lo);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    if out.contains(&0) {
        return Err(PgError::InvalidUrl(
            "NUL byte not allowed in connection URL",
        ));
    }
    String::from_utf8(out).map_err(|_| PgError::InvalidUrl("invalid UTF-8 in percent-escape"))
}

fn hex_nibble(byte: Option<u8>) -> Result<u8, PgError> {
    match byte {
        Some(b'0'..=b'9') => Ok(byte.unwrap() - b'0'),
        Some(b'a'..=b'f') => Ok(byte.unwrap() - b'a' + 10),
        Some(b'A'..=b'F') => Ok(byte.unwrap() - b'A' + 10),
        _ => Err(PgError::InvalidUrl("invalid percent-encoding")),
    }
}
