use anyhow::Result;
use std::{future::Future, pin::Pin};

use crate::domain::acp_session::{AcpSessionLookup, AcpSessionRecord};

pub trait AcpSessionStore: Send + Sync + 'static {
    fn record_session<'a>(
        &'a self,
        record: AcpSessionRecord,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

    fn latest_session<'a>(
        &'a self,
        lookup: AcpSessionLookup,
    ) -> Pin<Box<dyn Future<Output = Result<Option<AcpSessionRecord>>> + Send + 'a>>;
}
