from dora import Node

node = Node()

for i in range (100): 
    event = node.next()
    event_type = event["type"]
    if event_type == "INPUT":
        node.send_output("random", "random", event["metadata"])
    elif event_type == "STOP":
        print("received stop")
        break
    else:
        print("received unexpected event:", event_type)
        break