# Simulation Controller
This is simulation controller that can be used to visualize and interact with the underlying network. 
Here's a preview of the simulation controller:
![Preview of the simulation controller](/assets/gui_preview.png)

## Usage
The following sections, will explain how to interact with the simulation controller, showing the list of features and how to use them.
### Add sender
To add a sender, first of all select a node in the network. The selected node will have its color changed to white. On the bottom side there will be a label containing the ID of the selected node. Beneath it, the input field will allow you to specify the ID of the node that will be attached.  
By clicking the `Add sender` button, a new edge will be created (if possible) between the selected node and the node with the specified ID. Otherwise, an error message will be displayed.
### Remove sender
To remove a sender, first select the edge connecting two nodes. On the bottom side there will be a label containing the ID of the selected edge. By clicking the `Remove edge` button, the edge will be removed. Otherwise, an error message will be displayed.
### Change PDR
To change the PDR of a drone, first select a drone in the network. On the right side will appear a panel containing an input field and a button. By clicking the button, the PDR of the selected drone will be changed to the value specified in the input field. Otherwise, an error message will be displayed.  
The PDR __must__ be a value between 0 and 1.
### Crash a drone
To crash a drone, first select a drone in the network. On the right panel there will be a button labeled `Crash`. By clicking the button, the selected drone will be crashed. Otherwise, an error message will be displayed.
### Spawn a new drone
On the bottom of the right panel there will be a button labeled `Add drone`. By clicking the button, a new drone will be spawned in the network. The drone will be selected randomly between the ones bought by our group, also it will be placed in a random position and with no connections.
### Interact with a text server
To interact with a text server, first select a `Web Client` node. On the right you should see something like this:
![Web client panel](/assets/web_widget.png)
1. Under the `Ask for Server types` label, click Send. This will send a command to the client to ask for the available server types.  
When the client will respond, the `Server types` label will be updated with the available server types.
2. Then you can ask for the files available on a specific server, by writing in the input field the server's ID.  
If the validation is successful, the `Received files` list will be updated with the available files on the specified server. Otherwise, an error message will be displayed.
### Interact with a chat server
To interact with a chat server, first select a `Chat Client` node.
1. Under the `Ask for Server types` label, click Send. This will send a command to the client to ask for the available server types.  
When the client will respond, the `Server types` label will be updated with the available server types.
2. Click on a server, this will open a chat window.
3. To better understand how the chat mechanism works, write `/help` in the chat window. This will display the available commands.