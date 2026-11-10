//! Immutable working directory, environment, and output budget for one native worker.
//! Modified for Ariember/fastctx: resolve native tool budgets from the host environment.

use std::cell::RefCell;
use std::env::VarError;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::Arc;

thread_local! {
    static ACTIVE_ENVIRONMENT: RefCell<Option<Arc<SessionEnvironment>>> = const { RefCell::new(None) };
}

/// Native process state captured when a worker starts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionEnvironment {
    cwd: PathBuf,
    variables: Vec<(OsString, OsString)>,
}

impl SessionEnvironment {
    /// Captures the active tool environment, or the current process state outside a request.
    pub fn capture() -> Result<Self, String> {
        if let Some(environment) = ACTIVE_ENVIRONMENT.with(|slot| slot.borrow().clone()) {
            return Ok((*environment).clone());
        }
        let cwd = std::env::current_dir()
            .map_err(|error| format!("Cannot determine the session working directory: {error}."))?;
        Ok(Self::new(cwd, std::env::vars_os().collect()))
    }

    /// Builds an immutable snapshot from native values supplied by the host.
    pub fn new(cwd: PathBuf, variables: Vec<(OsString, OsString)>) -> Self {
        Self { cwd, variables }
    }

    /// Returns the worker's working directory.
    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    /// Looks up one native environment value using the platform's variable-name rules.
    pub fn var_os(&self, name: &str) -> Option<OsString> {
        self.variables
            .iter()
            .rev()
            .find(|(candidate, _)| environment_name_eq(candidate, name))
            .map(|(_, value)| value.clone())
    }

    /// Looks up one UTF-8 environment value with `std::env::var`-compatible failures.
    pub fn var(&self, name: &str) -> Result<String, VarError> {
        match self.var_os(name) {
            Some(value) => value.into_string().map_err(VarError::NotUnicode),
            None => Err(VarError::NotPresent),
        }
    }

    /// Runs synchronous work with the snapshot's working directory and budgets.
    pub fn activate<R>(self: &Arc<Self>, operation: impl FnOnce() -> R) -> R {
        let _scope = SessionEnvironmentScope::install(Arc::clone(self));
        operation()
    }
}

/// Immutable state owned by one native worker.
#[derive(Clone, Debug)]
pub struct SessionContext {
    /// The exact cwd and environment supplied by the host.
    pub environment: Arc<SessionEnvironment>,
}

impl SessionContext {
    /// Captures one native worker's environment.
    pub fn capture() -> Result<Arc<Self>, String> {
        Ok(Self::from_environment(SessionEnvironment::capture()?))
    }

    /// Uses host budgets without reading standalone FastCtx or Codex configuration files.
    pub fn from_environment(environment: SessionEnvironment) -> Arc<Self> {
        Arc::new(Self {
            environment: Arc::new(environment),
        })
    }

    /// Best-effort context for direct library constructors that cannot return startup errors.
    pub(crate) fn library_default() -> Arc<Self> {
        Self::capture().unwrap_or_else(|_| {
            Self::from_environment(SessionEnvironment::new(
                PathBuf::from("."),
                std::env::vars_os().collect(),
            ))
        })
    }

    /// Runs synchronous tool work with this worker's cwd and response budgets.
    pub(crate) fn activate<R>(&self, operation: impl FnOnce() -> R) -> R {
        self.environment.activate(operation)
    }
}

struct SessionEnvironmentScope {
    previous: Option<Arc<SessionEnvironment>>,
}

impl SessionEnvironmentScope {
    fn install(environment: Arc<SessionEnvironment>) -> Self {
        let previous = ACTIVE_ENVIRONMENT.with(|slot| slot.borrow_mut().replace(environment));
        Self { previous }
    }
}

impl Drop for SessionEnvironmentScope {
    fn drop(&mut self) {
        ACTIVE_ENVIRONMENT.with(|slot| *slot.borrow_mut() = self.previous.take());
    }
}

/// Returns the active worker cwd, or the process cwd outside a tool request.
pub(crate) fn current_dir() -> std::io::Result<PathBuf> {
    ACTIVE_ENVIRONMENT.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|environment| Ok(environment.cwd.clone()))
            .unwrap_or_else(std::env::current_dir)
    })
}

/// Returns the active worker variable, or the process variable outside a tool request.
pub(crate) fn var(name: &str) -> Result<String, VarError> {
    ACTIVE_ENVIRONMENT.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|environment| environment.var(name))
            .unwrap_or_else(|| std::env::var(name))
    })
}

fn environment_name_eq(candidate: &OsStr, expected: &str) -> bool {
    #[cfg(windows)]
    {
        candidate
            .to_str()
            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(expected))
    }
    #[cfg(not(windows))]
    {
        candidate == OsStr::new(expected)
    }
}

#[cfg(test)]
mod tests {
    use super::{SessionContext, SessionEnvironment};
    use std::path::PathBuf;
    use std::sync::Arc;

    fn environment(cwd: &str, budget: &str) -> Arc<SessionEnvironment> {
        Arc::new(SessionEnvironment::new(
            PathBuf::from(cwd),
            vec![("FASTCTX_TOKEN_BUDGET".into(), budget.into())],
        ))
    }

    #[test]
    fn snapshots_keep_host_budgets_and_restore_nested_scopes() {
        let first = environment("first", "9000");
        let second = environment("second", "4096");
        first.activate(|| {
            let session = SessionContext::library_default();
            assert_eq!(session.environment.cwd(), PathBuf::from("first"));
            assert_eq!(crate::budget::token_budget().unwrap(), 9000);
            second.activate(|| {
                assert_eq!(super::current_dir().unwrap(), PathBuf::from("second"));
                assert_eq!(crate::budget::token_budget().unwrap(), 4096);
                session.activate(|| {
                    assert_eq!(crate::budget::token_budget().unwrap(), 9000);
                });
                assert_eq!(crate::budget::token_budget().unwrap(), 4096);
            });
            assert_eq!(crate::budget::token_budget().unwrap(), 9000);
        });
    }

    #[test]
    fn invalid_host_budget_is_reported_by_the_tool() {
        let session = SessionContext::from_environment((*environment("cwd", "invalid")).clone());
        session.activate(|| {
            let error = crate::budget::token_budget().unwrap_err();
            assert!(error.contains("FASTCTX_TOKEN_BUDGET"));
        });
    }
}
