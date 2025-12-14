# ffengine GUI JSON (.gui) Format Specification v1.0

## Root Structure
| Key      | Type        | Description                                     |
|----------|-------------|-------------------------------------------------|
| guis     | `[GUI]`     | List of all GUI layouts                         |
| scripts  | `[script]`  | List of all scripts                             |
| fonts    | `[font]`    | List of all fonts                               |
| elements | `[element]` | List of all non-implicitly defined elements     |
| nodes    | `[node]`    | List of all root and initially unparented nodes |

### GUI Structure
| Key   | Type                 | Description                                 |
|-------|----------------------|---------------------------------------------|
| name  | `String`             | Name of GUI layout                          |
| nodes | `[integer]`          | List of used root node indices for this GUI |

### Script Structure
| Key                       | Type      | Description                   |
|---------------------------|-----------|-------------------------------|
| uri                       | `String`  | Path to the scripts .lua file |

### Element Structure
| Key              | Type          | Description                                       |
|------------------|---------------|---------------------------------------------------|
| name             | `String`      | Name of element                                   |
| type             | `String`      | Type of element. One of Quad, Text, Image         |
| info             | `ElementInfo` | Info about element. Unused properties are ignored |

#### ElementInfo Structure
| Key                 | Type         | Description                                                          |
|---------------------|--------------|----------------------------------------------------------------------|
| uri                 | `String`     | uri of the file to be used by the element                            |
| color               | `[float; 4]` | The base color of a quad or text element                             |
| additive_tint       | `[float; 4]` | The color that will be added to quads or images                      |
| multiplicative_tint | `[float; 4]` | The factor that the final quad or image color will be multiplied by  |
| corner radius       | `float`      | The radius in pixels of corners of quads and images to be rounded by |
| font                | `integer`    | Index of the font to be used by the text                             |
| text                | `String`     | Text to be displayed                                                 |
| font_size           | `float`      | Font size of the text, always in pixels                              |
| newline_distance    | `float`      | Distance in pixels before the text should auto wrap                  |

## Node Structure
| Key                      | Type                      | Description                                                                                                             |
|--------------------------|---------------------------|-------------------------------------------------------------------------------------------------------------------------|
| name                     | `String`                  | Name of the node                                                                                                        |
| interactable_information | `InteractableInformation` | Refer to InteractableInformation Structure                                                                              |
| container                | `Container`               | The nodes own container type and information                                                                            |
| parent_relation          | `ParentRelation`          | Information about how the node should act relative to its parent                                                        |
| width                    | `Size`                    | Nodes width                                                                                                             |
| height                   | `Size`                    | Nodes height                                                                                                            |
| children                 | `[Integer OR Node]`       | Nodes children. Either implicitly defined within the list, or an index to a root node in the overarching root node list |
| elements                 | `[Integer OR Element]`    | Nodes elements. Either implicitly defined within the list, or an index to an element in the overarching element list    |

### Container Structure
| Key  | Type            | Description                               |
|------|-----------------|-------------------------------------------|
| type | `String`        | Type of the container. One of Dock, Stack |
| info | `ContainerInfo` | Information about the container           |

#### ContainerInfo Structure
| Key             | Type      | Description                                                                                                                                                                                                                                                             |
|-----------------|-----------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| horizontal      | `boolean` | Whether the stack container stacks horizontally vs vertically                                                                                                                                                                                                           |
| alignment       | `String`  | Where the stack should start. One of End (bottom or right), Start (top or left), Center (stack centered).                                                                                                                                                               |
| stack_direction | `String`  | Which direction the stack should go relative to its axis. One of Normal (from the start to the end (after alignment)), Reverse (from the end, to the start (after alignment)), Alternating (first child in center of final stack, alternate adding on either direction) |
| spacing         | `float`   | Spacing in pixels between stacked children                                                                                                                                                                                                                              |
| padding         | `Padding` | Padding information                                                                                                                                                                                                                                                     |

##### Padding Structure
| Key    | Type    | Description                      |
|--------|---------|----------------------------------|
| left   | `float` | Padding to the left, in pixels   |
| right  | `float` | Padding to the right, in pixels  |
| top    | `float` | Padding to the top, in pixels    |
| bottom | `float` | Padding to the bottom, in pixels |

### ParentRelation Structure
| Key  | Type                 | Description                                                                                                           |
|------|----------------------|-----------------------------------------------------------------------------------------------------------------------|
| type | `String`             | Type of the relation. One of Independent (ignore parents container), Docking (used if parents container mode is Dock) |
| info | `ParentRelationInfo` | Information about the parent relation                                                                                 |

#### ParentRelationInfo Structure
| Key      | Type     | Description                                                                                                                                                                               |
|----------|----------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| relative | `bool`   | Should the node be positioned relative to the parent or not                                                                                                                               |
| anchor   | `String` | If the node is independent, which corner should it match to which of its parents corners. One of TopLeft, TopCenter, TopRight, Right, BottomRight, BottomCenter, BottomLeft, Left, Center |
| mode     | `String` | Docking mode, used if type is Dock. One of Top, Left, Right, Bottom                                                                                                                       |
| offset_x | `Offset` | offset in x direction                                                                                                                                                                     |
| offset_y | `Offset` | offset in y direction                                                                                                                                                                     |

##### Offset Structure
| Key   | Type     | Description                                                        |
|-------|----------|--------------------------------------------------------------------|
| type  | `String` | Type of the offset. One of Pixels or Factor                        |
| value | `float`  | The value of the offset. Either in pixels or factor of parent size |

### Size Structure
| Key  | Type       | Description                                           |
|------|------------|-------------------------------------------------------|
| type | `String`   | Type of the size. One of Factor, FillFactor, Absolute |
| info | `SizeInfo` | Information about the size                            |

#### SizeInfo Structure
| Key    | Type    | Description                                                                                                                                                            |
|--------|---------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| factor | `float` | Either the factor of the parents size to use, or the factor of the remaining space in the parent to fill after all other children have allocated space (if FillFactor) |
| pixels | `float` | Size of the node in pixels. Used if type is Absolute                                                                                                                   |

### Method Structure
| Key              | Type      | Description                                                       |
|------------------|-----------|-------------------------------------------------------------------|
| method           | `String`  | Name of the method.                                               |
| script           | `integer` | Index of the script the method is defined in.                     |
| hitbox_diversion | `integer` | Index of child node for the interactable hitbox to be diverted to |

### InteractableInformation Structure

| Key                | Type        | Description                                                                                                                  |
|--------------------|-------------|------------------------------------------------------------------------------------------------------------------------------|
| passive_actions    | `[Method]`  | Methods to be executed constantly by the node                                                                                |
| hover_actions      | `[Method]`  | Methods to be executed when the mouse is hovering over the hitbox                                                            |
| unhover_actions    | `[Method]`  | Methods to be executed when the mouse is not hovering over the hitbox (intended to undo the hover action)                    |
| left_down_actions  | `[Method]`  | Methods to be executed upon the left mouse being first pressed on the node                                                   |
| left_up_actions    | `[Method]`  | Methods to be executed upon the left mouse being released from the node. Only activates if mouse is still hovering the node  |
| right_down_actions | `[Method]`  | Methods to be executed upon the right mouse being first pressed on the node                                                  |
| right_up_actions   | `[Method]`  | Methods to be executed upon the right mouse being released from the node. Only activates if mouse is still hovering the node |
| left_hold_actions  | `[Method]`  | Methods to be executed whenever the left mouse button is pressed and hovering over the hitbox                                |
| right_hold_actions | `[Method]`  | Methods to be executed whenever the right mouse button is pressed and hovering over the hitbox                               |