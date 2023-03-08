package iam

type DefaultRole string

const (
	Guest     DefaultRole = "guest"
	Developer DefaultRole = "developer"
	Owner     DefaultRole = "owner"
)

type Role struct {
	permissions []Permission
}

func (r *Role) HasPermission(action PermissionAction, resource PermissionResource) bool {
	for _, p := range r.permissions {
		for _, r := range p.Resources {
			if r == resource {
				for _, a := range p.Actions {
					if a == action {
						return true
					}
				}
			}
		}
	}
	return false
}
