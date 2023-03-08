package iam

type usersMap map[string]*User

type User struct {
	Username       string
	HashedPassword Password
	Role           Role
}

type UserInput struct {
	Username       string
	HashedPassword string
	Role           Role
}

func NewUser(in *UserInput) *User {
	return &User{}
}

func (u *User) HasPermission(action PermissionAction, resource PermissionResource) bool {
	return u.Role.HasPermission(action, resource)
}
