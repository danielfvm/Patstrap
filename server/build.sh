#!/bin/bash

# Create virtual environment and install requirements
python -m venv patstrap-server-env
source patstrap-server-env/bin/activate 
pip install -r requirements.txt

# Use pyinstaller to create a standalone LINUX executable at ./dist/patstrap-server/patstrap-server
pyinstaller gui.py -n patstrap-server --noconfirm --onedir \
	--path=./patstrap-server-env/lib/python3.11/site-packages \
	--add-data "./global.css:." \
	--hidden-import zeroconf._utils.ipaddress \
	--hidden-import zeroconf._handlers.answers
