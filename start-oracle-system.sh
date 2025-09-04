#!/bin/bash

# Start Oracle System for BTC Options API
# This script starts the aggregator server and multiple oracle nodes

echo "Starting Oracle System..."
echo "========================"
echo ""

# Check if gnome-terminal is available
if command -v gnome-terminal &> /dev/null; then
    # Use gnome-terminal for GUI systems
    
    # Start aggregator
    echo "Starting aggregator server..."
    gnome-terminal --tab --title="Aggregator Server" -- bash -c "cd /home/zeno/projects/oracle-node/aggregator-server && nix-shell --run 'echo \"Aggregator Server starting on :50051...\"; cargo run'; read -p \"Press enter to close...\""
    
    # Wait for aggregator to start
    echo "Waiting for aggregator to start..."
    sleep 5
    
    # Start oracle nodes
    echo "Starting oracle nodes..."
    gnome-terminal --tab --title="Oracle Node 1" -- bash -c "cd /home/zeno/projects/oracle-node && nix-shell --run 'echo \"Oracle Node 1 starting...\"; cargo run -- --node-id node1 --aggregator-url http://localhost:50051'; read -p \"Press enter to close...\""
    
    gnome-terminal --tab --title="Oracle Node 2" -- bash -c "cd /home/zeno/projects/oracle-node && nix-shell --run 'echo \"Oracle Node 2 starting...\"; cargo run -- --node-id node2 --aggregator-url http://localhost:50051'; read -p \"Press enter to close...\""
    
    gnome-terminal --tab --title="Oracle Node 3" -- bash -c "cd /home/zeno/projects/oracle-node && nix-shell --run 'echo \"Oracle Node 3 starting...\"; cargo run -- --node-id node3 --aggregator-url http://localhost:50051'; read -p \"Press enter to close...\""
    
elif command -v tmux &> /dev/null; then
    # Use tmux for terminal multiplexing
    
    # Create new tmux session
    tmux new-session -d -s oracle-system
    
    # Create panes for each component
    tmux rename-window -t oracle-system:0 'Oracle System'
    
    # Aggregator in first pane
    tmux send-keys -t oracle-system:0.0 'cd /home/zeno/projects/oracle-node/aggregator-server && nix-shell --run "cargo run"' C-m
    
    # Split horizontally for first oracle node
    tmux split-window -h -t oracle-system:0
    tmux send-keys -t oracle-system:0.1 'cd /home/zeno/projects/oracle-node && nix-shell --run "cargo run -- --node-id node1 --aggregator-url http://localhost:50051"' C-m
    
    # Split vertically for second oracle node
    tmux split-window -v -t oracle-system:0.0
    tmux send-keys -t oracle-system:0.2 'cd /home/zeno/projects/oracle-node && nix-shell --run "cargo run -- --node-id node2 --aggregator-url http://localhost:50051"' C-m
    
    # Split vertically for third oracle node
    tmux split-window -v -t oracle-system:0.1
    tmux send-keys -t oracle-system:0.3 'cd /home/zeno/projects/oracle-node && nix-shell --run "cargo run -- --node-id node3 --aggregator-url http://localhost:50051"' C-m
    
    # Attach to session
    tmux attach-session -t oracle-system
    
else
    # Fallback: provide manual instructions
    echo "Neither gnome-terminal nor tmux found."
    echo ""
    echo "Please manually start the oracle system in separate terminals:"
    echo ""
    echo "Terminal 1 (Aggregator):"
    echo "  cd /home/zeno/projects/oracle-node/aggregator-server"
    echo "  nix-shell"
    echo "  cargo run"
    echo ""
    echo "Terminal 2 (Oracle Node 1):"
    echo "  cd /home/zeno/projects/oracle-node"
    echo "  nix-shell"
    echo "  cargo run -- --node-id node1 --aggregator-url http://localhost:50051"
    echo ""
    echo "Terminal 3 (Oracle Node 2):"
    echo "  cd /home/zeno/projects/oracle-node"
    echo "  nix-shell"
    echo "  cargo run -- --node-id node2 --aggregator-url http://localhost:50051"
    echo ""
    echo "Terminal 4 (Oracle Node 3):"
    echo "  cd /home/zeno/projects/oracle-node"
    echo "  nix-shell"
    echo "  cargo run -- --node-id node3 --aggregator-url http://localhost:50051"
fi

echo ""
echo "Oracle system startup initiated!"
echo "Check the terminal tabs/panes for status."
echo ""
echo "To stop the system:"
echo "- With gnome-terminal: Close each tab"
echo "- With tmux: Press Ctrl+B, then type ':kill-session' and press Enter"