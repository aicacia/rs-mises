#![forbid(unsafe_code)]

use std::{error::Error, fmt, process};

use tokio::process::Command;

#[derive(Debug, Clone, Copy)]
pub enum BiometricStrength {
  Weak,
  Strong,
}

#[derive(Debug, Clone)]
pub struct Policy {
  pub biometrics: Option<BiometricStrength>,
  pub password: bool,
  pub companion: bool,
}

#[derive(Default, Debug)]
pub struct PolicyBuilder {
  biometrics: Option<BiometricStrength>,
  password: bool,
  companion: bool,
}

impl PolicyBuilder {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn biometrics(mut self, b: Option<BiometricStrength>) -> Self {
    self.biometrics = b;
    self
  }

  pub fn password(mut self, v: bool) -> Self {
    self.password = v;
    self
  }

  pub fn companion(mut self, v: bool) -> Self {
    self.companion = v;
    self
  }

  pub fn build(self) -> Result<Policy, PolicyError> {
    Ok(Policy {
      biometrics: self.biometrics,
      password: self.password,
      companion: self.companion,
    })
  }
}

#[derive(Debug, Clone)]
pub struct AndroidText {
  pub title: String,
  pub subtitle: Option<String>,
  pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WindowsText {
  pub title: String,
  pub description: String,
}

impl WindowsText {
  pub fn new(title: impl Into<String>, description: impl Into<String>) -> Self {
    Self {
      title: title.into(),
      description: description.into(),
    }
  }
}

#[derive(Debug, Clone)]
pub struct Text {
  pub android: AndroidText,
  pub apple: String,
  pub windows: WindowsText,
}

#[derive(Debug)]
pub enum PolicyError {}

impl fmt::Display for PolicyError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "policy error")
  }
}

impl Error for PolicyError {}

#[cfg_attr(feature = "thiserror", derive(thiserror::Error))]
#[derive(Debug)]
pub enum AuthError {
  #[cfg_attr(
    feature = "thiserror",
    error("native authentication not supported on this platform")
  )]
  NotSupported,

  #[cfg_attr(feature = "thiserror", error("polkit tooling (pkcheck) not available"))]
  MissingTool,

  #[cfg_attr(feature = "thiserror", error("execution error: {0}"))]
  ExecutionError(String),
}

#[cfg(not(feature = "thiserror"))]
impl fmt::Display for AuthError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      AuthError::NotSupported => write!(f, "native authentication not supported on this platform"),
      AuthError::MissingTool => write!(f, "polkit tooling (pkcheck) not available"),
      AuthError::ExecutionError(s) => write!(f, "execution error: {}", s),
    }
  }
}

#[cfg(not(feature = "thiserror"))]
impl Error for AuthError {}

#[derive(Debug, Clone)]
pub struct Context;

impl Context {
  pub fn new<T>(_t: T) -> Self {
    Context
  }

  pub async fn authenticate(&self, _text: Text, _policy: &Policy) -> Result<(), AuthError> {
    if cfg!(not(target_os = "linux")) {
      return Err(AuthError::NotSupported);
    }

    if which::which("pkcheck").is_err() {
      return Err(AuthError::MissingTool);
    }

    let status = Command::new("pkcheck")
      .arg("--action-id")
      .arg("org.freedesktop.policykit.exec")
      .arg("--allow-user-interaction")
      .arg("--process")
      .arg(format!("{}", process::id()))
      .status()
      .await;

    match status {
      Ok(s) if s.success() => Ok(()),
      Ok(s) => Err(AuthError::ExecutionError(format!(
        "pkcheck exited with {}",
        s
      ))),
      Err(e) => Err(AuthError::ExecutionError(format!(
        "failed to spawn pkcheck: {}",
        e
      ))),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::BiometricStrength;
  use super::PolicyBuilder;

  #[test]
  fn policy_builder_works() {
    let p = PolicyBuilder::new()
      .biometrics(Some(BiometricStrength::Strong))
      .password(false)
      .companion(false)
      .build()
      .unwrap();
    assert!(matches!(p.biometrics, Some(BiometricStrength::Strong)));
  }

  #[tokio::test]
  #[cfg(not(target_os = "linux"))]
  async fn authenticate_not_supported() {
    use super::{AndroidText, AuthError, Context, Text, WindowsText};

    let ctx = Context::new(());
    let text = Text {
      android: AndroidText {
        title: "t".into(),
        subtitle: None,
        description: None,
      },
      apple: "a".into(),
      windows: WindowsText::new("t", "d"),
    };
    let policy = PolicyBuilder::new().build().unwrap();
    let res = ctx.authenticate(text, &policy).await.unwrap_err();
    assert!(matches!(res, AuthError::NotSupported));
  }
}
