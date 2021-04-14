# Install tools via `apt-get`
sudo apt-get --yes update \
  && sudo DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends --yes \
  bison \
  cmake \
  curl \
  flex \
  g++ \
  gcc \
  git \
  gpg-agent \
  libssl-dev \
  lsb-release \
  make \
  ninja-build \
  patch \
  pkg-config \
  python-is-python3 \
  software-properties-common \
  wget \
  zlib1g \
  zlib1g-dev
