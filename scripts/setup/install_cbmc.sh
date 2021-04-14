# Install CBMC 5.27
wget https://github.com/diffblue/cbmc/releases/download/cbmc-5.27.0/ubuntu-20.04-cbmc-5.27.0-Linux.deb \
  && sudo dpkg -i ubuntu-20.04-cbmc-5.27.0-Linux.deb \
  && cbmc --version
