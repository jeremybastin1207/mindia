package iam

import "mindia/utils"

type UserManager struct {
	users usersMap
}

func NewUserManager() *UserManager {
	return &UserManager{}
}

func (m *UserManager) ReadAll() ([]*User, error) {
	return utils.ToArray(m.users), nil
}
