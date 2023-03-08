package iam

type PermissionAction string

const (
	ActionAll    PermissionAction = "*"
	ActionCreate PermissionAction = "create"
	ActionRead   PermissionAction = "read"
	ActionUpdate PermissionAction = "update"
	ActionDelete PermissionAction = "delete"
)

type PermissionResource string

const (
	ResourceAll    PermissionResource = "*"
	ResourceFile   PermissionResource = "file"
	ResourcePolicy PermissionResource = "policy"
	ResourceFolder PermissionResource = "folder"
	ResourceUser   PermissionResource = "user"
)

type Permission struct {
	Actions   []PermissionAction
	Resources []PermissionResource
}

type PermissionInput struct {
	Actions   []PermissionAction
	Resources []PermissionResource
}

func NewPermission(in *PermissionInput) *Permission {
	return &Permission{
		Actions:   in.Actions,
		Resources: in.Resources,
	}
}
