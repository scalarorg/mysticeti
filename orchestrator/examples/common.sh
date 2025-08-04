# Create a new file: orchestrator/examples/common_functions.sh
#!/bin/bash

# Function to prompt for input with default value
prompt_with_default() {
    local prompt="$1"
    local default="$2"
    local var_name="$3"
    
    if [ -z "$default" ]; then
        read -p "$prompt: " input
    else
        read -p "$prompt [$default]: " input
    fi
    
    # Use default if input is empty
    if [ -z "$input" ]; then
        input="$default"
    fi
    
    eval "$var_name=\"$input\""
}

# Function to prompt for yes/no with default
prompt_yes_no() {
    local prompt="$1"
    local default="$2"
    local var_name="$3"
    
    while true; do
        if [ "$default" = "y" ]; then
            read -p "$prompt [Y/n]: " input
        else
            read -p "$prompt [y/N]: " input
        fi
        
        # Use default if input is empty
        if [ -z "$input" ]; then
            input="$default"
        fi
        
        case $input in
            [Yy]* ) eval "$var_name=true"; break;;
            [Nn]* ) eval "$var_name=false"; break;;
            * ) echo "Please answer y or n.";;
        esac
    done
}
