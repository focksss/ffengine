# ffengine GUI JSON (.gui) Format Specification v1.0

### Common Keys
| Key               | Type         | Description                                                                                                                     |
|-------------------|--------------|---------------------------------------------------------------------------------------------------------------------------------|
| position          | `[float; 2]` | Position of object relative to parent.<br>Normalized to the parents scale by default,<br>in pixels if absolute_position is true |
| scale             | `[float; 2]` | Scale of an object.<br>Normalized to the parents scale by default,<br>in pixels if absolute_scale is true                       |
| clip_min          | `[float; 2]` | Clipping bound minimum of the object.<br/>Normalized to parents scale                                                           |
| clip_max          | `[float; 2]` | Clipping bound maximum of the object.<br/>Normalized to parents scale                                                           |
| absolute_position | `bool`       | Refer to description of position                                                                                                |
| absolute_scale    | `bool`       | Refer to description of scale                                                                                                   |
| color             | `[float; 4]` | Color of the object in RGBA, normalized                                                                                         |

## Root Structure
| Key     | Type       | Description             |
|---------|------------|-------------------------|
| guis    | `[GUI]`    | List of all GUI layouts |
| scripts | `[script]` | List of all scripts     |
| fonts   | `[font]`   | List of all fonts       |
| quads   | `[quad]`   | List of all quads       |
| texts   | `[text]`   | List of all texts       |
| nodes   | `[node]`   | List of all nodes       |

### GUI Structure
| Key   | Type                 | Description                                           |
|-------|----------------------|-------------------------------------------------------|
| name  | `String`             | Name of GUI layout                                    |
| nodes | `[integer]`          | List of all node root node indices for the GUI layout |

### Script Structure
| Key                       | Type      | Description                   |
|---------------------------|-----------|-------------------------------|
| uri                       | `String`  | Path to the scripts .lua file |

### Font Structure
| Key                       | Type      | Description                                            |
|---------------------------|-----------|--------------------------------------------------------|
| uri                       | `String`  | Path to the fonts .ttf file                            |
| glyph_msdf_size           | `integer` | Size in pixels of each glyphs region in the MSDF atlas |
| glyph_msdf_distance_range | `float`   | Distance range to be used in MSDF generation           |

### Quad Structure
| Key               | Type      | Description                                                 |
|-------------------|-----------|-------------------------------------------------------------|
| position          | Common    | Common                                                      |
| scale             | Common    | Common                                                      |
| clip_min          | Common    | Common                                                      |
| clip_max          | Common    | Common                                                      |
| absolute_position | Common    | Common                                                      |
| absolute_scale    | Common    | Common                                                      |
| color             | Common    | Common                                                      |
| corner_radius     | `integer` | The radius of the corner rounding to be applied to the quad |

### Text Structure
| Key               | Type              | Description                        |
|-------------------|-------------------|------------------------------------|
| text_information  | `TextInformation` | Refer to TextInformation Structure |
| position          | Common            | Common                             |
| scale             | Common            | Common                             |
| clip_min          | Common            | Common                             |
| clip_max          | Common            | Common                             |
| absolute_position | Common            | Common                             |
| absolute_scale    | Common            | Common                             |
| color             | Common            | Common                             |

#### TextInformation Structure
| Key              | Type      | Description                                         |
|------------------|-----------|-----------------------------------------------------|
| font             | `integer` | Index of the font to be used by the text            |
| text             | `String`  | Text to be displayed                                |
| font_size        | `float`   | Font size of the text, always in pixels             |
| newline_distance | `float`   | Distance in pixels before the text should auto wrap |

## Node Structure
| Key                      | Type                      | Description                                |
|--------------------------|---------------------------|--------------------------------------------|
| name                     | `String`                  | Name of the node                           |
| interactable_information | `InteractableInformation` | Refer to InteractableInformation Structure |
| children                 | `[integer]`               | Array of children indices                  |
| position                 | Common                    | Common                                     |
| scale                    | Common                    | Common                                     |
| absolute_position        | Common                    | Common                                     |
| absolute_scale           | Common                    | Common                                     |

#### InteractableInformation Structure

| Key                | Type        | Description                                                                                                               |
|--------------------|-------------|---------------------------------------------------------------------------------------------------------------------------|
| passive_actions    | `[integer]` | Indices of the scripts to be executed constantly by the node.                                                             |
| hover_actions      | `[integer]` | Indices of the scripts to be executed when the mouse is hovering over the hitbox.                                         |
| unhover_actions    | `[integer]` | Indices of the scripts to be executed when the mouse is not hovering over the hitbox (intended to undo the hover action). |
| left_tap_actions   | `[integer]` | Indices of the scripts to be executed upon a single left click of the hitbox.                                             |
| right_tap_actions  | `[integer]` | Indices of the scripts to be executed upon a single right click of the hitbox.                                            |
| left_hold_actions  | `[integer]` | Indices of the scripts to be executed whenever the left mouse button is pressed and hovering over the hitbox.             |
| right_hold_actions | `[integer]` | Indices of the scripts to be executed whenever the right mouse button is pressed and hovering over the hitbox.            |
| hitbox_diversion   | `integer`   | Index of child node for the interactable hitbox to be diverted to                                                         |
