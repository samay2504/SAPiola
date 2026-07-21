use std::time::SystemTime;
use tonic::{Request, Status, service::Interceptor};
use tracing::{info, warn};

#[derive(Clone, Default)]
pub struct AuditInterceptor;

impl Interceptor for AuditInterceptor {
    fn call(&mut self, request: Request<()>) -> Result<Request<()>, Status> {
        let tenant_id = request
            .metadata()
            .get("tenant-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("default")
            .to_string();

        let principal_id = request
            .metadata()
            .get("principal-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("anonymous")
            .to_string();

        // Very basic mock RBAC for audit logging purposes
        if principal_id == "anonymous" && tenant_id == "admin" {
            warn!(
                event = "audit_denied",
                tenant_id = %tenant_id,
                principal_id = %principal_id,
                reason = "Anonymous user attempted admin tenant access"
            );
            return Err(Status::permission_denied("Admin tenant access requires authentication"));
        }

        info!(
            event = "audit_allowed",
            tenant_id = %tenant_id,
            principal_id = %principal_id,
            timestamp = ?SystemTime::now(),
            "Inbound RPC request"
        );

        Ok(request)
    }
}
