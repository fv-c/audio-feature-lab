cmake_minimum_required(VERSION 3.20)

project(afl_essentia_wrapper LANGUAGES CXX)

set(ESSENTIA_PREFIX "${ESSENTIA_PREFIX}" CACHE PATH "Essentia installation prefix")
set(ESSENTIA_EXTRA_INCLUDE_DIRS "${ESSENTIA_EXTRA_INCLUDE_DIRS}" CACHE STRING "Additional include directories required by Essentia dependencies")

if(NOT ESSENTIA_PREFIX AND DEFINED ENV{ESSENTIA_PREFIX})
  set(ESSENTIA_PREFIX "$ENV{ESSENTIA_PREFIX}")
endif()

if(NOT ESSENTIA_PREFIX)
  message(FATAL_ERROR "ESSENTIA_PREFIX must point to an Essentia installation when building the native backend")
endif()

add_library(afl_essentia_wrapper STATIC
  src/afl_essentia_wrapper.cpp
)

target_compile_features(afl_essentia_wrapper PUBLIC cxx_std_17)
target_include_directories(afl_essentia_wrapper
  PUBLIC
    include
  PRIVATE
    ${ESSENTIA_PREFIX}/include
    ${ESSENTIA_PREFIX}/include/essentia
    ${ESSENTIA_EXTRA_INCLUDE_DIRS}
)

install(TARGETS afl_essentia_wrapper ARCHIVE DESTINATION lib)
