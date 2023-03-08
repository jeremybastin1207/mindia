package configurer

import (
	"fmt"
	"io/ioutil"
	"mindia/project"

	"gopkg.in/yaml.v3"
)

const fileName = "config.yaml"

type Configurer struct {
}

func NewConfigurer() *Configurer {
	return &Configurer{}
}

func (c *Configurer) PersistConfig(p *project.Project) error {
	yamlData, err := yaml.Marshal(projectToConfig(p))
	if err != nil {
		fmt.Printf("Error while Marshaling. %v", err)
		return err
	}

	err = ioutil.WriteFile(fileName, yamlData, 0644)
	if err != nil {
		fmt.Printf("Unable to write data into the file. %v", err)
	}
	return nil
}
