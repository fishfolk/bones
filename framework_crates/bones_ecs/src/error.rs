use std::error::Error;

/// The types of errors used throughout the ECS.
// TODO: Re-evaluate `EcsError` variants.
// Some these error variants may not be used anymore. Also, I think most of the times
// that we return `EcsError`, there is only one possible error that could occur for that function.
// If that is the case in all situations, we should consider breaking each error type into it's
// own struct, so that we aren't returning an enum with a bunch of errors that will never happen
// for each function call.
#[derive(Debug, thiserror::Error)]
pub enum EcsError {
    /// A resource was not initialized in the [`World`][crate::World] but the
    /// [`System`][crate::system::System] tries to access it.
    #[error("Resource or component not initialized")]
    NotInitialized,
    /// The requested resource is already borrowed.
    ///
    /// This error is created if the `System` tries to read a resource that has already been mutably
    /// borrowed. It can also happen when trying to mutably borrow a resource that is already being
    /// read.
    ///
    /// This error should not occur during normal use, as the dispatchers can recover easily.
    #[error("Resource or component already borrowed")]
    AlreadyBorrowed,
    /// The execution of the dispatcher failed and returned one or more errors.
    #[error("Dispatcher failed with one or more errors: {0:?}")]
    DispatcherExecutionFailed(Vec<anyhow::Error>),
    /// This variant is for user-defined errors.
    ///
    /// To create an error of this type easily, use the `system_error!` macro.
    #[error("System errored: {0}")]
    SystemError(Box<dyn Error + Send>),
}

/// The result of a `System`'s execution.
pub type SystemResult<Out = ()> = anyhow::Result<Out>;
