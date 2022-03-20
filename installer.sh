#!/bin/bash

RESTORE='\033[0m'
RED='\033[00;31m'
GREEN='\033[00;32m'
YELLOW='\033[00;33m'
BLUE='\033[00;34m'

echo -e "$RED
   _ _ _ _           _     _____     _   _ ___    
  | | | |_|___ ___ _| |___|   | |___| |_|_|  _|_ _ 
  | | | | |  _| -_| . |___| | | | . |  _| |  _| | |
  |_____|_|_| |___|___|   |_|___|___|_| |_|_| |_  |
                                              |___|
"

echo -e "$GREEN Wired Notify notification daemon installer for RHEL-based GNU/Linux distributions (Fedora, CentOS, Scientific Linux, etc...) that use DNF as package manager.\n $RESTORE"

if [ "$(id -un)" != "root" ]; then 
    echo -e "$RED [ X ] Sudo permissions required! $RESTORE";
    exit 1 
fi

function notificationDaemonCompilation(){
    echo -e "$BLUE [ * ] Cloning the Wired-Notify repository. $RESTORE"
    git clone https://github.com/Toqozz/wired-notify.git /home/$(ls -U /home/ | head -1)/wired-notify > /dev/null 2>&1
    echo -e "$GREEN [ ✔ ] Wired-Notify repository successfully cloned in /home/$(ls -U /home/ | head -1)/wired-notify!\n $RESTORE"
    echo -e "$BLUE [ * ] Starting compilation! This process may take a few minutes. $RESTORE"
    cd /home/$(ls -U /home/ | head -1)/wired-notify/
    cargo build --release > /dev/null 2>&1
    echo -e "$GREEN [ ✔ ] Wired-Notify has been successfully compiled! To execute run the command : '~/wired-notify/target/release/wired &' or move the wired binary to another path and run: '/path/to/wired &'. $RESTORE "
    exit 0
}

function checkDunstFacilities(){
    which dunst > /dev/null 2>&1 
        if [ "$?" -eq "0" ]; then
            read -p "$(echo -e "$YELLOW [ ! ] An installation of Dunst has been found, which may cause problems with the FreeDesktop notification service. Do you want to uninstall Dunst? (y)es or (n)o: $RESTORE")" INPUT
            case $INPUT in
                [Yy]* )
                    echo -e "$BLUE [ * ] Uninstalling Dunst... $RESTORE";
                    dnf -y remove dunst > /dev/null 2>&1;
                    echo -e "$GREEN [ ✔ ] Dunst has been successfully uninstalled!\n $RESTORE";
                    notificationDaemonCompilation;;
                [Nn]* )
                    echo -e "$YELLOW [ ! ] Warning: This option may cause unexpected problems with Wired-Notify. $RESTORE";
                    notificationDaemonCompilation
            esac
        else
            notificationDaemonCompilation
        fi
}

function startDaemonCompilation(){
    which git > /dev/null 2>&1 
    if [ "$?" -eq "0" ]; then
        checkDunstFacilities
    else
        read -p "$(echo -e "$RED [ X ] Git is required to clone the Wired-Notify repository! Do you want to install it? (y)es, (n)ot: $RESTORE")" INPUT
        case $INPUT in
            [Yy]* )
                echo -e "$BLUE [ * ] Installing Git! $RESTORE";
                dnf -y install git > /dev/null 2>&1;
                echo -e "$GREEN [ ✔ ]$BLUE Git ➜$GREEN INSTALLED\n $RESTORE";
                checkDunstFacilities;;
            [Nn]* )
                echo -e "$RED [ X ] The Wired-Notify repository cannot be cloned without Git. Exiting the installer... $RESTORE";
                exit 1;;
        esac
    fi
}

function dependenciesInstallation(){
    echo -e "$BLUE [ * ] Installing dependencies $RESTORE"
    sleep 1
    dnf -y install cargo rust-x11+xss-devel rust-glib+v2_68-devel dbus-devel pkgconf-pkg-config rust-pango+default-devel rust-cairo-rs+default-devel >/dev/null 2>&1
    which cargo > /dev/null 2>&1
    if [ "$?" -eq "0" ]; then
        echo -e "$GREEN [ ✔ ]$BLUE Cargo and dependencies ➜$GREEN INSTALLED.\n $RESTORE"
        startDaemonCompilation
        sleep 1
    else
        echo -e "$RED [ X ]$BLUE Cargo and dependencies: ➜$RED NOT INSTALLED.\n $RESTORE $BLUE Try to install the dependencies manually using the command: "dnf install cargo rust-x11+xss-devel rust-glib+v2_68-devel dbus-devel pkgconf-pkg-config rust-pango+default-devel rust-cairo-rs+default-devel" and re-run the script! $RESTORE"
        exit 1
    fi 
}

function connectionCheck(){
    echo -e "$BLUE [ * ] Checking for internet connection $RESTORE"
    sleep 1
    echo -e "GET http://google.com HTTP/1.0\n\n" | nc google.com 80 > /dev/null 2>&1
    if [ $? -ne 0 ]; then
        echo -e "$RED [ X ]$BLUE Internet Connection ➜$RED OFFLINE! $RESTORE";
        echo -e "$BLUE Network connection is required for Wired-Notify installation. $RESTORE"
        exit 0
    else
        echo -e "$GREEN [ ✔ ]$BLUE Internet Connection ➜$GREEN CONNECTED!\n";
        dependenciesInstallation
        sleep 1
    fi
}

connectionCheck