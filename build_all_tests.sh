#!/bin/bash

set -e # Bail on error.

CARGO="/home/fredrik/.cargo/bin/cargo"
ROOT_DIR="/home/fredrik/bin/mymake"
MYMAKE="$ROOT_DIR/target/debug/mymake"
CWD=`pwd`

build_mymake()
{
   echo "Building latest version of MyMake."
   cd $ROOT_DIR
   $CARGO build
}

test_mymake_minimal_build()
{
   TEST_DIR="$ROOT_DIR/test_project"
   mkdir $TEST_DIR && cd $TEST_DIR
   mkdir "$TEST_DIR/source"

   cat << EOF > $TEST_DIR/source/test.cpp
#include <iostream>

int main()
{
   std::cout << "Minimum build test successful!\n";
}

EOF

cat << EOF > $TEST_DIR/run.mmk
MMK_EXECUTABLE:
   x

MMK_SOURCES:
   source/test.cpp
EOF

   cd $ROOT_DIR && build_mymake
   mkdir "$ROOT_DIR/build" && cd "$ROOT_DIR/build"
   $MYMAKE -g "$TEST_DIR/run.mmk" && $ROOT_DIR/build/release/x
   build_result=$?
   if [ "$build_result" -ne 0 ]; then
      return "$build_result"
   fi
   cd "$ROOT_DIR"
   rm -rf "$ROOT_DIR/build" "$ROOT_DIR/test.cpp" "$ROOT_DIR/run.mmk" "$ROOT_DIR/test_project"
}

execute_command() 
{
   $@
   if [ $? -gt 0 ]; then 
      echo "$@ FAILED. Aborting..."
      exit 1
   fi
}

cargo_test()
{
   path=$1
   cd $path
   echo "cargo test -p $path"
   execute_command "$CARGO test"
   cd $ROOT_DIR
}

[ $CWD != $ROOT_DIR ] && cd $ROOT_DIR

cargo_test "${ROOT_DIR}/mmk_parser"
cargo_test "${ROOT_DIR}/builder"
cargo_test "${ROOT_DIR}/dependency"
cargo_test "${ROOT_DIR}/generator"

test_mymake_minimal_build

if [ "$?" -eq 0 ]; then
   echo "SUCCESS"
else
   echo "FAILURE"
fi
exit "$?"
