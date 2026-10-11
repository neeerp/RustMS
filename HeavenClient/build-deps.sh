#!/bin/bash
set -ex

SOURCE_DIR=${PWD}
CORES=$(nproc)

mkdir -p libs

# Pinned dependency revisions. These are the exact commits the client is known to build against.
# (openal-soft and glfw were already pinned to short hashes that match these.)
ALURE_REV=2b54c0d8b074ff1b84821c791556b3634c11764f
OPENAL_SOFT_REV=f5e0eef34db3a3ab94b61a2f99f84f078ba947e7
GLAD_REV=b5d260b65b3edbbd9e75f5a6c9924fe0ff4f04dd
LZ4_REV=384eb158358a21f4c501cebc2cadd127cbd4fde2
NOLIFENX_REV=8af296265f6ce4bffecd96f35323952e820fb45b
GLFW_REV=0a49ef0a00baa3ab520ddc452f0e3b1e099c5589
ASIO_REV=180bc40ea03ae95bf45dd6890e979ea9ffef54e8
STB_REV=b42009b3b9d4ca35bc703f5310eedc74f584be58

# fetch & build alure
if [[
    ! -d "libs/alure/include" ||
    ! -f "libs/alure/build/libalure2.so" 
]]; then
    rm -rf "libs/alure"
    echo "fetching and building alure..."
    cd libs && \
    git clone https://github.com/kcat/alure && \
    cd alure && \
    git checkout $ALURE_REV && \
    cd build && \
    cmake .. && \
    make -j$CORES
    echo "successfully compiled alure"
    cd $SOURCE_DIR
fi


# fetch & build openal-soft
if [[
    ! -d "libs/openal-soft/include" ||
    ! -f "libs/openal-soft/build/libopenal.so"
]]; then
    rm -rf "libs/openal-soft"
    echo "fetching and building openal-soft..."
    cd libs && \
    git clone https://github.com/kcat/openal-soft && \
    cd openal-soft && \
    git checkout $OPENAL_SOFT_REV && \
    cd build && \
    cmake .. && \
    make -j$CORES
    echo "successfully compiled openal-soft"
    cd $SOURCE_DIR
fi


# fetch & build glad
if [[
    ! -d "libs/glad/build/include" ||
    ! -f "libs/glad/build/libglad.a"
]]; then
    rm -rf "libs/glad"
    echo "fetching and building glad..."
    cd libs && \
    git clone https://github.com/Dav1dde/glad.git && \
    cd glad && \
    git checkout $GLAD_REV && \
    mkdir build && \
    cd build && \
    cmake .. && \
    make -j$CORES
    echo "successfully compiled glad"
    cd $SOURCE_DIR
fi

# fetch & build lz4
if [[
    ! -f "libs/lz4/lib/liblz4.a"
]]; then
    rm -rf "libs/lz4"
    echo "fetching and building lz4..."
    cd libs && \
    git clone https://github.com/lz4/lz4.git && \
    cd lz4 && \
    git checkout $LZ4_REV && \
    make -j$CORES && \
    echo "successfully compiled lz4"
    cd $SOURCE_DIR
fi

# fetch & build NoLifeNx
if [[
    ! -f "libs/NoLifeNx/nlnx/build/libNoLifeNx.a"
]]; then
    rm -rf "libs/NoLifeNx"
    echo "fetching and building NoLifeNx..."
    cd libs && \
    mkdir NoLifeNx && \
    cd NoLifeNx && \
    git clone https://github.com/lain3d/NoLifeNx nlnx && \
    cd nlnx && \
    git checkout $NOLIFENX_REV && \
    mkdir build && \
    cd build && \
    cmake .. && \
    make -j$CORES && \
    echo "successfully compiled NoLifeNx"
    cd $SOURCE_DIR
fi


# fetch & build glfw
if [[
    ! -d "libs/glfw/include" ||
    ! -f "libs/glfw/build/src/libglfw3.a"
]]; then
    rm -rf "libs/glfw"
    echo "fetching and building glfw..."
    cd libs && \
    git clone https://github.com/glfw/glfw && \
    cd glfw && \
    git checkout $GLFW_REV && \
    mkdir build && \
    cd build && \
    cmake .. && \
    make -j$CORES && \
    echo "successfully compiled glfw"
    cd $SOURCE_DIR
fi


# fetch asio
if [[
    ! -d "libs/asio/asio/include"
]]; then
    rm -rf "libs/asio"
    echo "fetching asio..."
    cd libs && \
    git clone https://github.com/chriskohlhoff/asio.git && \
    git -C asio checkout $ASIO_REV && \
    echo "successfully fetched asio"
    cd $SOURCE_DIR
fi

# fetch stb
if [[
    ! -d "libs/stb"
]]; then
    rm -rf "libs/stb"
    echo "fetching stb..."
    cd libs && \
    git clone https://github.com/nothings/stb.git && \
    git -C stb checkout $STB_REV && \
    echo "successfully fetched stb"
    cd $SOURCE_DIR
fi

