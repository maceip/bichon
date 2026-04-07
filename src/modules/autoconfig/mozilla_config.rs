#![allow(dead_code)] // Mirrors upstream autoconfig public API surface.

//
// Mozilla Thunderbird autoconfig XML types (config file format).
// Derived from the MIT-licensed `autoconfig` crate:
// https://github.com/Dust-Mail/autoconfig (v0.4.0, src/config.rs)
//

use std::time::Duration;

use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    version: String,
    email_provider: EmailProvider,
    #[serde(rename = "oAuth2")]
    oauth2: Option<OAuth2Config>,
}

impl Config {
    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn email_provider(&self) -> &EmailProvider {
        &self.email_provider
    }

    pub fn oauth2(&self) -> Option<&OAuth2Config> {
        self.oauth2.as_ref()
    }
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OAuth2Config {
    issuer: String,
    scope: String,
    #[serde(rename = "authURL")]
    auth_url: String,
    #[serde(rename = "tokenURL")]
    token_url: String,
}

impl OAuth2Config {
    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    pub fn scope(&self) -> Vec<&str> {
        self.scope.split(' ').collect()
    }

    pub fn auth_url(&self) -> &str {
        &self.auth_url
    }

    pub fn token_url(&self) -> &str {
        &self.token_url
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct EmailProvider {
    id: String,
    #[serde(rename = "$value")]
    properties: Vec<EmailProviderProperty>,
}

impl EmailProvider {
    pub fn properties(&self) -> &Vec<EmailProviderProperty> {
        &self.properties
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn domain(&self) -> Vec<&str> {
        let mut domains: Vec<&str> = Vec::new();
        for property in &self.properties {
            if let EmailProviderProperty::Domain(domain) = property {
                domains.push(domain);
            }
        }
        domains
    }

    pub fn display_name(&self) -> Option<&str> {
        for property in &self.properties {
            if let EmailProviderProperty::DisplayName(display_name) = property {
                return Some(display_name);
            }
        }
        None
    }

    pub fn display_short_name(&self) -> Option<&str> {
        for property in &self.properties {
            if let EmailProviderProperty::DisplayShortName(short_name) = property {
                return Some(short_name);
            }
        }
        None
    }

    pub fn incoming_servers(&self) -> Vec<&Server> {
        let mut servers: Vec<&Server> = Vec::new();
        for property in &self.properties {
            if let EmailProviderProperty::IncomingServer(server) = property {
                servers.push(server);
            }
        }
        servers
    }

    pub fn outgoing_servers(&self) -> Vec<&Server> {
        let mut servers: Vec<&Server> = Vec::new();
        for property in &self.properties {
            if let EmailProviderProperty::OutgoingServer(server) = property {
                servers.push(server);
            }
        }
        servers
    }

    pub fn servers(&self) -> Vec<&Server> {
        let mut servers: Vec<&Server> = Vec::new();
        for property in &self.properties {
            match property {
                EmailProviderProperty::IncomingServer(server) => servers.push(server),
                EmailProviderProperty::OutgoingServer(server) => servers.push(server),
                _ => {}
            }
        }
        servers
    }

    pub fn documentation(&self) -> Option<&Documentation> {
        for property in &self.properties {
            if let EmailProviderProperty::Documentation(documentation) = property {
                return Some(documentation);
            }
        }
        None
    }
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum EmailProviderProperty {
    Domain(String),
    DisplayName(String),
    DisplayShortName(String),
    IncomingServer(Server),
    OutgoingServer(Server),
    Documentation(Documentation),
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Server {
    r#type: ServerType,
    #[serde(rename = "$value")]
    properties: Vec<ServerProperty>,
}

impl Server {
    pub fn properties(&self) -> &Vec<ServerProperty> {
        &self.properties
    }

    pub fn server_type(&self) -> &ServerType {
        &self.r#type
    }

    pub fn hostname(&self) -> Option<&str> {
        for property in &self.properties {
            if let ServerProperty::Hostname(hostname) = property {
                return Some(hostname);
            }
        }
        None
    }

    pub fn port(&self) -> Option<&u16> {
        for property in &self.properties {
            if let ServerProperty::Port(port) = property {
                return Some(port);
            }
        }
        None
    }

    pub fn security_type(&self) -> Option<&SecurityType> {
        for property in &self.properties {
            if let ServerProperty::SocketType(socket_type) = property {
                return Some(socket_type);
            }
        }
        None
    }

    pub fn authentication_type(&self) -> Vec<&AuthenticationType> {
        let mut types: Vec<&AuthenticationType> = Vec::new();
        for property in &self.properties {
            if let ServerProperty::Authentication(authentication_type) = property {
                types.push(authentication_type);
            }
        }
        types
    }

    pub fn username(&self) -> Option<&str> {
        for property in &self.properties {
            if let ServerProperty::Username(username) = property {
                return Some(username);
            }
        }
        None
    }

    pub fn password(&self) -> Option<&str> {
        for property in &self.properties {
            if let ServerProperty::Password(password) = property {
                return Some(password);
            }
        }
        None
    }
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ServerProperty {
    Hostname(String),
    Port(u16),
    #[serde(rename = "socketType")]
    SocketType(SecurityType),
    Authentication(AuthenticationType),
    OwaURL(String),
    EwsURL(String),
    UseGlobalPreferredServer(bool),
    Pop3(Pop3Config),
    Username(String),
    Password(String),
}

#[derive(Debug, Deserialize, PartialEq)]
pub enum SecurityType {
    #[serde(rename = "plain")]
    Plain,
    #[serde(rename = "STARTTLS")]
    Starttls,
    #[serde(rename = "SSL")]
    Tls,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ServerType {
    Exchange,
    Imap,
    Pop3,
    Smtp,
}

#[derive(Debug, Deserialize, PartialEq)]
pub enum AuthenticationType {
    #[serde(rename = "password-cleartext")]
    PasswordCleartext,
    #[serde(rename = "password-encrypted")]
    PasswordEncrypted,
    #[serde(rename = "NTLM")]
    Ntlm,
    #[serde(rename = "GSAPI")]
    GsApi,
    #[serde(rename = "client-IP-address")]
    ClientIPAddress,
    #[serde(rename = "TLS-client-cert")]
    TlsClientCert,
    OAuth2,
    #[serde(rename = "None")]
    None,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Pop3Config {
    leave_messages_on_server: bool,
    download_on_biff: Option<bool>,
    days_to_leave_messages_on_server: Option<u64>,
    check_interval: Option<CheckInterval>,
}

impl Pop3Config {
    pub fn leave_messages_on_server(&self) -> &bool {
        &self.leave_messages_on_server
    }

    pub fn download_on_biff(&self) -> Option<&bool> {
        self.download_on_biff.as_ref()
    }

    pub fn time_to_leave_messages_on_server(&self) -> Option<Duration> {
        match self.days_to_leave_messages_on_server.as_ref() {
            Some(days) => Some(Duration::from_secs(days * 24 * 60 * 60)),
            None => None,
        }
    }

    pub fn check_interval(&self) -> Option<Duration> {
        match self.check_interval.as_ref() {
            Some(interval) => {
                if let Some(minutes) = interval.minutes {
                    return Some(Duration::from_secs(minutes * 60));
                }
                None
            }
            None => None,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq)]
struct CheckInterval {
    minutes: Option<u64>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Documentation {
    url: String,
    #[serde(rename = "$value")]
    properties: Vec<DocumentationDescription>,
}

impl Documentation {
    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn properties(&self) -> &Vec<DocumentationDescription> {
        &self.properties
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct DocumentationDescription {
    lang: Option<String>,
    #[serde(rename = "$value")]
    description: String,
}

impl DocumentationDescription {
    pub fn language(&self) -> Option<&str> {
        self.lang.as_deref()
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}
