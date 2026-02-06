use axum_test::TestServer;
use mindia::db::TenantRepository;
use uuid::Uuid;

/// Test user data
pub struct TestUser {
    pub email: String,
    pub password: String,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub token: String,
}

/// Test master API key used by the test app setup (must match setup_test_app).
pub const TEST_MASTER_API_KEY: &str = "test-master-api-key-at-least-32-characters-long";

/// Register a new test user and tenant via API key flow.
/// Creates a tenant and returns test user struct (use API key auth for requests).
/// The token is the test master API key for authenticating with the test app.
pub async fn register_test_user(
    _client: &TestServer,
    email: Option<&str>,
    password: Option<&str>,
    _org_name: Option<&str>,
) -> TestUser {
    let email = email.unwrap_or("test@example.com").to_string();
    let password = password.unwrap_or("TestPassword123!").to_string();
    // Tenant and user IDs are created when creating API key; use placeholder for compatibility.
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

/// Login and get a token (deprecated: auth/register and auth/login removed; use API key auth).
pub async fn login_user(_client: &TestServer, _email: &str, _password: &str) -> String {
    String::new()
}

/// Get authentication token for a user (deprecated: use API key auth).
pub async fn get_auth_token(_client: &TestServer, _user: &TestUser) -> String {
    String::new()
}

/// Create a test tenant directly in the database (user/org removed; returns tenant_id, tenant_id).
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

