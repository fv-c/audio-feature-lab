cmake_minimum_required(VERSION 3.20)

project(afl_essentia_wrapper LANGUAGES CXX)

add_library(afl_essentia_wrapper STATIC
  src/afl_essentia_wrapper.cpp
)

target_compile_features(afl_essentia_wrapper PUBLIC cxx_std_17)
target_include_directories(afl_essentia_wrapper PUBLIC include)

# TODO(local-essentia): add Essentia include directories and libraries here.
# TODO(local-essentia): switch this wrapper from scaffold mode to real extraction mode once
# the local Essentia toolchain is available on the target platform.

