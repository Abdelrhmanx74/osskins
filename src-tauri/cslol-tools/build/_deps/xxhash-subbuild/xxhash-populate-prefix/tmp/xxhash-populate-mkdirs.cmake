# Distributed under the OSI-approved BSD 3-Clause License.  See accompanying
# file LICENSE.rst or https://cmake.org/licensing for details.

cmake_minimum_required(VERSION ${CMAKE_VERSION}) # this file comes with cmake

# If CMAKE_DISABLE_SOURCE_CHANGES is set to true and the source directory is an
# existing directory in our source tree, calling file(MAKE_DIRECTORY) on it
# would cause a fatal error, even though it would be a no-op.
if(NOT EXISTS "C:/Users/Mana/Documents/work-space/fuck-exalted/src-tauri/cslol-manager/cslol-tools/build/_deps/xxhash-src")
  file(MAKE_DIRECTORY "C:/Users/Mana/Documents/work-space/fuck-exalted/src-tauri/cslol-manager/cslol-tools/build/_deps/xxhash-src")
endif()
file(MAKE_DIRECTORY
  "C:/Users/Mana/Documents/work-space/fuck-exalted/src-tauri/cslol-manager/cslol-tools/build/_deps/xxhash-build"
  "C:/Users/Mana/Documents/work-space/fuck-exalted/src-tauri/cslol-manager/cslol-tools/build/_deps/xxhash-subbuild/xxhash-populate-prefix"
  "C:/Users/Mana/Documents/work-space/fuck-exalted/src-tauri/cslol-manager/cslol-tools/build/_deps/xxhash-subbuild/xxhash-populate-prefix/tmp"
  "C:/Users/Mana/Documents/work-space/fuck-exalted/src-tauri/cslol-manager/cslol-tools/build/_deps/xxhash-subbuild/xxhash-populate-prefix/src/xxhash-populate-stamp"
  "C:/Users/Mana/Documents/work-space/fuck-exalted/src-tauri/cslol-manager/cslol-tools/build/_deps/xxhash-subbuild/xxhash-populate-prefix/src"
  "C:/Users/Mana/Documents/work-space/fuck-exalted/src-tauri/cslol-manager/cslol-tools/build/_deps/xxhash-subbuild/xxhash-populate-prefix/src/xxhash-populate-stamp"
)

set(configSubDirs Debug)
foreach(subDir IN LISTS configSubDirs)
    file(MAKE_DIRECTORY "C:/Users/Mana/Documents/work-space/fuck-exalted/src-tauri/cslol-manager/cslol-tools/build/_deps/xxhash-subbuild/xxhash-populate-prefix/src/xxhash-populate-stamp/${subDir}")
endforeach()
if(cfgdir)
  file(MAKE_DIRECTORY "C:/Users/Mana/Documents/work-space/fuck-exalted/src-tauri/cslol-manager/cslol-tools/build/_deps/xxhash-subbuild/xxhash-populate-prefix/src/xxhash-populate-stamp${cfgdir}") # cfgdir has leading slash
endif()
