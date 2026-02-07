use axum_test::TestServer;
use mindia_db::TenantRepository;
use uuid::Uuid;

/// Test user data (API key auth: token is the master API key).
pub struct TestUser {
    pub email: String,
    pub password: String,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub token: String,
}

/// Test master API key (must match setup_test_app).
pub const TEST_MASTER_API_KEY: &str = "test-master-api-key-at-least-32-characters-long";

/// Register a test user; returns TestUser with token = master API key for requests.
pub async fn register_test_user(
    _client: &TestServer,
    email: Option<&str>,
    password: Option<&str>,
    _org_name: Option<&str>,
) -> TestUser {
    let email = email.unwrap_or("test@example.com").to_string();
    let password = password.unwrap_or("TestPassword123!").to_string();
    let tenant_id = Uuid::nil();
    let user_id = Uuid::nil();
    TestUser {
        email,
        password,
        tenant_id,
        user_id,
        token: TEST_MASTER_API_KEY.to_string(),
    }
}

/// Create a test tenant in the DB; returns (tenant_id, tenant_id).
pub async fn create_test_user_in_db(
    pool: &sqlx::PgPool,
    tenant_id: Option<Uuid>,
    _email: Option<&str>,
    _password: Option<&str>,
) -> (Uuid, Uuid) {
    let tenant_repo = TenantRepository::new(pool.clone());
    let tenant_id = if let Some(id) = tenant_id {
        id
    } else {
        let tenant = tenant_repo
            .create_tenant("Test Tenant")
            .await
            .expect("Failed to create test tenant");
        tenant.id
    };
    (tenant_id, tenant_id)
}
