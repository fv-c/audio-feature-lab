#ifndef AFL_ESSENTIA_WRAPPER_H
#define AFL_ESSENTIA_WRAPPER_H

#ifdef __cplusplus
extern "C" {
#endif

char* afl_essentia_backend_version(void);
char* afl_essentia_analyze_file(const char* path, const char* config_json);
void afl_essentia_free_string(char* value);

#ifdef __cplusplus
}
#endif

#endif
