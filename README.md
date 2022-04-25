# SecureFileEditor
#### Created by Jarod Gillliam, Xin He, Hunter McGarity, and Chenqian Xu
A good file editor makes a significant difference in terms of usability, portability, and productivity for many software developers. While large-scale editor projects, such as Microsoft Visual Studio Code, provide a plethora of additional features that some developers require, they can become bloated with an abundance of plugins and confusing project management. It is the goal of this team to develop a simple file/text editor focused on usability and convenience in the Rust language.

# Docker Instructions (Release 2)
For the second release, we are simply aiming for a bash shell on container startup. The user will need to manually cd into the 'src' directory from here and then
run the program with "cargo run [filename]".

With Docker installed:
1) cd SecureFileEditor
2) "docker build -t [image_name] ."
    ex: "docker build -t file_editor ."
3) upon successful build, use "docker images" to see a list of all built images to ensure yours was built
4) "docker run -it [image_name]"
   The docker container should launch in the /home/src folder; if it doesn't, cd into the src folder
5) "cargo run text.txt" [other file names if applicable]
6) Press "Ctrl + h" for a list of keyboard commands

## V1
(Set to release 3/4/2022)
For the first release, the team plans to focus on the base functionality of the editor, such as opening, saving, and creating a new file. These features are facets of any file editor and so we feel it is important to focus on them first and foremost.
* [x] Open a File
* [x] Edit a File
* [x] Save File

Crates used:
   - crossterm = "0.23.0"

## V2
(Set to release 4/8/2022)
For the second release, we plan to focus on more advanced features such as the find/find and replace features, as well as various on-screen information such as displaying the name and type of the open file. 
* [x] Find
* [x] Find and Replace
* [x] Detailed file information

Crates used:
   - crossterm = "0.23.0"
   - regex = "1.5.5"
   - chrono = "0.4.0"
   - unicode-width = "0.1.9"
   - unicode-truncate = "0.2.0"

## V3
(Set to release 4/26/2022)
For the final release, the team will focus on ironing out issues with previous releases and implementing other advanced features which are quality of life improvements like syntax highlighting and keyboard shortcuts.
* [ ] Keyboard Shortcuts
* [ ] Command Line
* [ ] Syntax Highlighting
