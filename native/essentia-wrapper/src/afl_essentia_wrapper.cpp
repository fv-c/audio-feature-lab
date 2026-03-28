#include "afl_essentia_wrapper.h"

#include <cstring>
#include <string>

namespace {

char* duplicate_string(const std::string& value) {
  char* buffer = new char[value.size() + 1];
  std::memcpy(buffer, value.c_str(), value.size() + 1);
  return buffer;
}

}  // namespace

char* afl_essentia_backend_version(void) {
  // TODO(local-essentia): return the real Essentia backend version string once linked.
  return duplicate_string("essentia-unconfigured");
}

char* afl_essentia_analyze_file(const char* path, const char* config_json) {
  // TODO(local-essentia): replace this scaffold response with a real Essentia analysis call.
  // The wrapper contract remains a single JSON string result per file.
  if (path == nullptr || config_json == nullptr) {
    return duplicate_string("{\"status\":\"error\",\"message\":\"null input\"}");
  }

  return duplicate_string(
      "{\"status\":\"error\",\"message\":\"Essentia wrapper scaffold only; local integration is not configured\"}");
}

void afl_essentia_free_string(char* value) {
  delete[] value;
}
