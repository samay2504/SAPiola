pub struct TenantScopedKey<'a> {
    pub tenant_id: &'a str,
    pub inner_key: &'a [u8],
}

impl<'a> TenantScopedKey<'a> {
    pub fn new(tenant_id: &'a str, inner_key: &'a [u8]) -> Self {
        Self {
            tenant_id,
            inner_key,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        [self.tenant_id.as_bytes(), &[0u8], self.inner_key].concat()
    }
}
