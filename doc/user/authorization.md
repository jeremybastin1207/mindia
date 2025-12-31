# Authorization

Mindia implements role-based access control (RBAC) with three user roles. This guide explains the permission model and how to work with user roles.

## Table of Contents

- [User Roles](#user-roles)
- [Permissions Matrix](#permissions-matrix)
- [Checking Permissions](#checking-permissions)
- [Role-Based UI](#role-based-ui)
- [Best Practices](#best-practices)

## User Roles

Mindia has three built-in roles per tenant:

### Admin

**Full control** over the organization's data and settings.

**Can**:
- ✅ Upload, view, update, delete all media
- ✅ Create, manage, delete webhooks
- ✅ View analytics and audit logs
- ✅ Manage users (future feature)
- ✅ Update organization settings (future feature)
- ✅ Access all API endpoints

**Use Cases**:
- Organization owners
- IT administrators
- Power users who manage the system

### Member

**Read and write access** to media, but cannot manage webhooks or settings.

**Can**:
- ✅ Upload media files
- ✅ View all media
- ✅ Update their own uploads (if implemented)
- ✅ Delete their own uploads (if implemented)
- ✅ Use semantic search
- ✅ View basic analytics

**Cannot**:
- ❌ Manage webhooks
- ❌ View detailed audit logs
- ❌ Manage users or settings

**Use Cases**:
- Content creators
- Regular users
- Application service accounts

### Viewer

**Read-only access** to media.

**Can**:
- ✅ View media files
- ✅ Download media
- ✅ Use semantic search
- ✅ View basic analytics

**Cannot**:
- ❌ Upload files
- ❌ Delete files
- ❌ Manage webhooks
- ❌ View detailed audit logs

**Use Cases**:
- External partners
- Auditors
- Read-only integrations

## Permissions Matrix

| Permission | Admin | Member | Viewer |
|-----------|-------|--------|--------|
| **Media Operations** |
| Upload images/videos/audio/documents | ✅ | ✅ | ❌ |
| View/download media | ✅ | ✅ | ✅ |
| Delete media | ✅ | ✅* | ❌ |
| Transform images (resize, etc.) | ✅ | ✅ | ✅ |
| Stream videos (HLS) | ✅ | ✅ | ✅ |
| **Search & Discovery** |
| Semantic search | ✅ | ✅ | ✅ |
| List/browse media | ✅ | ✅ | ✅ |
| **Webhooks** |
| Create webhooks | ✅ | ❌ | ❌ |
| View webhooks | ✅ | ❌ | ❌ |
| Update webhooks | ✅ | ❌ | ❌ |
| Delete webhooks | ✅ | ❌ | ❌ |
| View webhook events | ✅ | ❌ | ❌ |
| **Analytics** |
| View storage summary | ✅ | ✅ | ✅ |
| View traffic statistics | ✅ | ✅ | ❌ |
| View audit logs | ✅ | ❌ | ❌ |
| Export analytics | ✅ | ❌ | ❌ |
| **User Management** (future) |
| Invite users | ✅ | ❌ | ❌ |
| Manage user roles | ✅ | ❌ | ❌ |
| Remove users | ✅ | ❌ | ❌ |

\* Members may only delete their own uploads (to be implemented)

## Checking Permissions

### Server-Side (Automatic)

Mindia automatically enforces permissions based on the JWT token's role. No additional code needed.

```bash
# Example: Member trying to create webhook
curl -X POST https://api.example.com/api/webhooks \
  -H "Authorization: Bearer $MEMBER_TOKEN" \
  -d '{"url": "https://example.com/hook", "events": ["image.uploaded"]}'

# Response: 403 Forbidden
{
  "error": "Insufficient permissions"
}
```

### Client-Side Permission Checks

Authentication is via **master API key** or **tenant API keys**. There is no user login or role endpoint; all authenticated requests have full access to the tenant's data. For UI/UX, you can restrict actions in your own application based on how you issue and manage API keys (e.g. one key per environment or per integration).

### Permission Helper Class

```javascript
class PermissionManager {
  constructor(role) {
    this.role = role;
  }

  isAdmin() {
    return this.role === 'admin';
  }

  isMember() {
    return this.role === 'member';
  }

  isViewer() {
    return this.role === 'viewer';
  }

  canWrite() {
    return this.isAdmin() || this.isMember();
  }

  canRead() {
    return true; // All roles can read
  }

  canManageWebhooks() {
    return this.isAdmin();
  }

  canViewAnalytics() {
    return true; // Basic analytics for all
  }

  canViewAuditLogs() {
    return this.isAdmin();
  }

  canUpload() {
    return this.canWrite();
  }

  canDelete() {
    return this.canWrite();
  }

  canManageUsers() {
    return this.isAdmin();
  }
}

// Usage
const permissions = new PermissionManager(user.role);

if (permissions.canUpload()) {
  // Show upload UI
}

if (permissions.canManageWebhooks()) {
  // Show webhooks section
}
```

## Role-Based UI

### React Example

```tsx
import { createContext, useContext, useEffect, useState } from 'react';

interface User {
  id: string;
  email: string;
  role: 'admin' | 'member' | 'viewer';
  tenant: {
    id: string;
    name: string;
  };
}

interface AuthContextType {
  user: User | null;
  permissions: PermissionManager | null;
  loading: boolean;
}

const AuthContext = createContext<AuthContextType>({
  user: null,
  permissions: null,
  loading: true,
});

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetchCurrentUser();
  }, []);

  async function fetchCurrentUser() {
    try {
      const token = localStorage.getItem('token');
      if (!token) {
        setLoading(false);
        return;
      }

      const response = await fetch('/api/auth/me', {
        headers: { 'Authorization': `Bearer ${token}` },
      });

      if (response.ok) {
        const userData = await response.json();
        setUser(userData);
      }
    } catch (error) {
      console.error('Failed to fetch user:', error);
    } finally {
      setLoading(false);
    }
  }

  const permissions = user ? new PermissionManager(user.role) : null;

  return (
    <AuthContext.Provider value={{ user, permissions, loading }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  return useContext(AuthContext);
}

// Permission-based component
export function UploadButton() {
  const { permissions } = useAuth();

  if (!permissions?.canUpload()) {
    return null; // Hide button for viewers
  }

  return <button onClick={handleUpload}>Upload File</button>;
}

// Conditional rendering
export function WebhooksSection() {
  const { permissions } = useAuth();

  if (!permissions?.canManageWebhooks()) {
    return (
      <div>
        <p>You don't have permission to manage webhooks.</p>
        <p>Contact your organization admin for access.</p>
      </div>
    );
  }

  return (
    <div>
      <h2>Webhooks</h2>
      {/* Webhook management UI */}
    </div>
  );
}

// Role badge component
export function RoleBadge({ role }: { role: string }) {
  const colors = {
    admin: 'bg-purple-100 text-purple-800',
    member: 'bg-blue-100 text-blue-800',
    viewer: 'bg-gray-100 text-gray-800',
  };

  return (
    <span className={`px-2 py-1 rounded text-sm ${colors[role]}`}>
      {role.charAt(0).toUpperCase() + role.slice(1)}
    </span>
  );
}
```

### Vue Example

```vue
<template>
  <div>
    <!-- Upload button (admin, member only) -->
    <button v-if="canUpload" @click="handleUpload">
      Upload File
    </button>

    <!-- Webhooks section (admin only) -->
    <div v-if="canManageWebhooks">
      <h2>Webhooks</h2>
      <!-- Webhook management UI -->
    </div>

    <!-- Read-only notice for viewers -->
    <div v-if="isViewer" class="alert">
      You have read-only access to this organization.
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue';

const user = ref(null);

const role = computed(() => user.value?.role);
const isAdmin = computed(() => role.value === 'admin');
const isMember = computed(() => role.value === 'member');
const isViewer = computed(() => role.value === 'viewer');

const canUpload = computed(() => isAdmin.value || isMember.value);
const canManageWebhooks = computed(() => isAdmin.value);

onMounted(async () => {
  // Authentication is via API key; there is no /api/auth/me. Use your API key for all requests.
  const apiKey = import.meta.env.VITE_MINDIA_API_KEY || '';
  if (apiKey) user.value = { role: 'admin' }; // All API key requests have full access
});

async function handleUpload() {
  // Upload logic
}
</script>
```

## Best Practices

### 1. Always Check Permissions Client-Side

Even though the server enforces permissions, check them in the UI to provide better UX:

```javascript
// ✅ Good: Check before showing UI
if (permissions.canUpload()) {
  showUploadButton();
} else {
  showUpgradeMessage();
}

// ❌ Bad: Show button, let server reject
// User clicks, waits, gets error - bad experience
```

### 2. Handle Permission Errors Gracefully

```javascript
async function createWebhook(data) {
  try {
    const response = await fetch('/api/webhooks', {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${token}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(data),
    });

    if (response.status === 403) {
      throw new Error('You don't have permission to create webhooks. Contact your admin.');
    }

    if (!response.ok) {
      throw new Error('Failed to create webhook');
    }

    return await response.json();
    
  } catch (error) {
    console.error('Webhook creation error:', error);
    showErrorToast(error.message);
  }
}
```

### 3. Display Role Information

```javascript
// Show user their role and what it means
function UserRoleInfo({ role }) {
  const roleDescriptions = {
    admin: 'Full access to all features and settings',
    member: 'Can upload and manage media files',
    viewer: 'Read-only access to media files',
  };

  const rolePermissions = {
    admin: ['Upload files', 'Delete files', 'Manage webhooks', 'View analytics', 'Manage users'],
    member: ['Upload files', 'Delete own files', 'View analytics'],
    viewer: ['View files', 'Download files', 'Search files'],
  };

  return (
    <div className="role-info">
      <h3>Your Role: {role}</h3>
      <p>{roleDescriptions[role]}</p>
      <h4>Permissions:</h4>
      <ul>
        {rolePermissions[role].map(perm => (
          <li key={perm}>{perm}</li>
        ))}
      </ul>
    </div>
  );
}
```

### 4. Provide Upgrade Paths

```javascript
// For viewers trying to access member/admin features
function UpgradePrompt({ requiredRole }) {
  const { user } = useAuth();

  if (user.role === 'viewer' && requiredRole !== 'viewer') {
    return (
      <div className="upgrade-prompt">
        <h3>Upgrade Required</h3>
        <p>This feature requires {requiredRole} access.</p>
        <button onClick={() => contactAdmin()}>
          Contact Administrator
        </button>
      </div>
    );
  }

  return null;
}
```

### 5. Audit Role Changes (Future Feature)

When user management is implemented:

```javascript
// Log role changes for security
async function changeUserRole(userId, newRole) {
  const response = await fetch(`/api/users/${userId}/role`, {
    method: 'PATCH',
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ role: newRole }),
  });

  if (response.ok) {
    // Log the change
    logAuditEvent({
      action: 'role_changed',
      userId,
      oldRole: user.role,
      newRole,
      changedBy: currentUser.id,
      timestamp: new Date().toISOString(),
    });
  }

  return response;
}
```

### 6. Implement Least Privilege

```javascript
// ✅ Good: Use member accounts for uploads
const uploaderToken = getMemberToken(); // Member role

// ❌ Bad: Use admin token for everything
const uploaderToken = getAdminToken(); // Unnecessary privileges
```

### 7. Role-Based API Keys (Future)

For service accounts and integrations:

```javascript
// Example: Read-only API key for external app
const apiKey = {
  key: 'mk_live_xxx',
  role: 'viewer',
  permissions: ['read:images', 'read:videos'],
  rateLimit: 1000, // requests/hour
};

// Upload service with write access
const uploadKey = {
  key: 'mk_live_yyy',
  role: 'member',
  permissions: ['write:images', 'write:videos', 'read:*'],
  rateLimit: 100,
};
```

## Security Considerations

### 1. Never Trust Client-Side Checks

Client-side permission checks are for UX only. Always enforce server-side:

```javascript
// ✅ Good: Server enforces, client enhances UX
if (permissions.canUpload()) {
  showUploadButton(); // Better UX
}
// Server will still reject if permission check is bypassed

// ❌ Bad: Client-only enforcement
if (permissions.canDelete()) {
  deleteFile(); // Can be bypassed
}
```

### 2. Validate Role in Token

```javascript
// Server-side validation
function validateToken(token) {
  const decoded = jwt.verify(token, JWT_SECRET);
  
  // Verify role is valid
  if (!['admin', 'member', 'viewer'].includes(decoded.role)) {
    throw new Error('Invalid role in token');
  }

  // Verify user still has this role (check DB)
  const user = await getUser(decoded.sub);
  if (user.role !== decoded.role) {
    throw new Error('Role mismatch - token may be stale');
  }

  return decoded;
}
```

### 3. Log Permission Denials

```javascript
// Log failed permission checks for security monitoring
app.use((req, res, next) => {
  const originalSend = res.send;
  
  res.send = function(data) {
    if (res.statusCode === 403) {
      logSecurityEvent({
        type: 'permission_denied',
        user: req.user?.id,
        role: req.user?.role,
        endpoint: req.path,
        method: req.method,
        ip: req.ip,
        timestamp: new Date().toISOString(),
      });
    }
    
    originalSend.call(this, data);
  };
  
  next();
});
```

## Next Steps

- [Authentication](authentication.md) - Learn about JWT tokens and login
- [Multi-Tenancy](multi-tenancy.md) - Understanding organization isolation
- [Webhooks](webhooks.md) - Set up event notifications (admin only)
- [Analytics](analytics.md) - View usage metrics

