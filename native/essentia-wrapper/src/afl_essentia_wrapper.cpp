#include "afl_essentia_wrapper.h"

#include <algorithm>
#include <cmath>
#include <cstring>
#include <iomanip>
#include <map>
#include <mutex>
#include <optional>
#include <sstream>
#include <string>
#include <string_view>
#include <vector>

#include <algorithmfactory.h>
#include <debugging.h>
#include <essentia.h>
#include <pool.h>
#include <utils/extractor_music/extractor_version.h>
#include <version.h>

namespace {

using essentia::Pool;
using essentia::Real;
using FamilyMap = std::map<std::string, std::string>;
using NestedMap = std::map<std::string, FamilyMap>;

struct BackendConfig {
  std::string profile;
  bool frame_level = false;
  std::vector<std::string> enabled_features;
  std::vector<std::string> statistics;
};

struct Payload {
  NestedMap features;
  NestedMap aggregation;
  NestedMap frame_level;
  std::vector<std::string> warnings;
  std::vector<std::string> errors;
  std::string status_code = "ok";
  bool success = true;
  bool frame_level_enabled = false;
};

char* duplicate_string(const std::string& value) {
  char* buffer = new char[value.size() + 1];
  std::memcpy(buffer, value.c_str(), value.size() + 1);
  return buffer;
}

void ensure_essentia_initialized() {
  static std::once_flag init_once;
  std::call_once(init_once, []() {
    essentia::init();
    essentia::infoLevelActive = false;
    essentia::warningLevelActive = false;
  });
}

std::string json_escape(std::string_view input) {
  std::string escaped;
  escaped.reserve(input.size() + 8);

  for (const char ch : input) {
    switch (ch) {
      case '\"':
        escaped += "\\\"";
        break;
      case '\\':
        escaped += "\\\\";
        break;
      case '\b':
        escaped += "\\b";
        break;
      case '\f':
        escaped += "\\f";
        break;
      case '\n':
        escaped += "\\n";
        break;
      case '\r':
        escaped += "\\r";
        break;
      case '\t':
        escaped += "\\t";
        break;
      default:
        if (static_cast<unsigned char>(ch) < 0x20) {
          std::ostringstream stream;
          stream << "\\u" << std::hex << std::setw(4) << std::setfill('0')
                 << static_cast<int>(static_cast<unsigned char>(ch));
          escaped += stream.str();
        } else {
          escaped.push_back(ch);
        }
        break;
    }
  }

  return escaped;
}

std::string json_string(std::string_view value) {
  return "\"" + json_escape(value) + "\"";
}

std::string json_number(double value) {
  if (!std::isfinite(value)) {
    return "null";
  }

  std::ostringstream stream;
  stream << std::setprecision(15) << value;
  return stream.str();
}

std::string json_array(const std::vector<std::string>& values) {
  std::string output = "[";
  for (std::size_t index = 0; index < values.size(); ++index) {
    if (index > 0) {
      output += ",";
    }
    output += json_string(values[index]);
  }
  output += "]";
  return output;
}

std::string json_array(const std::vector<Real>& values) {
  std::string output = "[";
  for (std::size_t index = 0; index < values.size(); ++index) {
    if (index > 0) {
      output += ",";
    }
    output += json_number(values[index]);
  }
  output += "]";
  return output;
}

std::string json_array(const std::vector<std::vector<Real>>& values) {
  std::string output = "[";
  for (std::size_t index = 0; index < values.size(); ++index) {
    if (index > 0) {
      output += ",";
    }
    output += json_array(values[index]);
  }
  output += "]";
  return output;
}

std::string render_object(const std::map<std::string, std::string>& fields) {
  std::string output = "{";
  bool first = true;

  for (const auto& [key, value] : fields) {
    if (!first) {
      output += ",";
    }
    first = false;
    output += json_string(key);
    output += ":";
    output += value;
  }

  output += "}";
  return output;
}

std::string render_family_collection(const NestedMap& values) {
  std::string output = "{";
  const std::vector<std::string> family_order = {
      "spectral", "temporal", "rhythm", "tonal", "dynamics", "metadata"};

  for (std::size_t index = 0; index < family_order.size(); ++index) {
    if (index > 0) {
      output += ",";
    }

    const std::string& family = family_order[index];
    output += json_string(family);
    output += ":";

    const auto family_it = values.find(family);
    if (family_it == values.end()) {
      output += "{}";
      continue;
    }

    output += render_object(family_it->second);
  }

  output += "}";
  return output;
}

std::string render_aggregation(const NestedMap& aggregation) {
  return render_family_collection(aggregation);
}

std::string render_family(const NestedMap& values, const std::string& family) {
  const auto family_it = values.find(family);
  if (family_it == values.end()) {
    return "{}";
  }

  return render_object(family_it->second);
}

std::string render_features(const Payload& payload) {
  std::map<std::string, std::string> fields;
  fields.emplace("spectral", render_family(payload.features, "spectral"));
  fields.emplace("temporal", render_family(payload.features, "temporal"));
  fields.emplace("rhythm", render_family(payload.features, "rhythm"));
  fields.emplace("tonal", render_family(payload.features, "tonal"));
  fields.emplace("dynamics", render_family(payload.features, "dynamics"));
  fields.emplace("metadata", render_family(payload.features, "metadata"));
  fields.emplace(
      "frame_level",
      payload.frame_level_enabled ? render_family_collection(payload.frame_level) : "null");
  return render_object(fields);
}

std::string render_status(const Payload& payload) {
  const std::string code =
      payload.success && !payload.warnings.empty() ? "partial" : payload.status_code;

  std::map<std::string, std::string> fields;
  fields.emplace("code", json_string(code));
  fields.emplace("errors", json_array(payload.errors));
  fields.emplace("success", payload.success ? "true" : "false");
  fields.emplace("warnings", json_array(payload.warnings));
  return render_object(fields);
}

std::string render_payload(const std::map<std::string, std::string>& audio_fields,
                           const Payload& payload) {
  std::map<std::string, std::string> top_level;
  top_level.emplace("aggregation", render_aggregation(payload.aggregation));
  top_level.emplace("audio", render_object(audio_fields));
  top_level.emplace("features", render_features(payload));
  top_level.emplace("status", render_status(payload));
  return render_object(top_level);
}

std::string error_payload(const std::string& code, const std::string& message) {
  Payload payload;
  payload.status_code = code;
  payload.success = false;
  payload.errors.push_back(message);
  return render_payload({}, payload);
}

bool extract_quoted_value(std::string_view source,
                          const std::string& key,
                          std::string& value) {
  const std::string marker = "\"" + key + "\":\"";
  const std::size_t start = source.find(marker);
  if (start == std::string_view::npos) {
    return false;
  }

  std::size_t cursor = start + marker.size();
  std::string parsed;

  while (cursor < source.size()) {
    const char current = source[cursor++];
    if (current == '\\') {
      if (cursor >= source.size()) {
        return false;
      }
      parsed.push_back(source[cursor++]);
      continue;
    }
    if (current == '"') {
      value = parsed;
      return true;
    }
    parsed.push_back(current);
  }

  return false;
}

bool extract_bool_value(std::string_view source, const std::string& key, bool& value) {
  const std::string marker = "\"" + key + "\":";
  const std::size_t start = source.find(marker);
  if (start == std::string_view::npos) {
    return false;
  }

  const std::size_t cursor = start + marker.size();
  if (source.substr(cursor, 4) == "true") {
    value = true;
    return true;
  }
  if (source.substr(cursor, 5) == "false") {
    value = false;
    return true;
  }
  return false;
}

bool extract_string_array(std::string_view source,
                          const std::string& key,
                          std::vector<std::string>& values) {
  const std::string marker = "\"" + key + "\":[";
  const std::size_t start = source.find(marker);
  if (start == std::string_view::npos) {
    return false;
  }

  std::size_t cursor = start + marker.size();
  values.clear();

  while (cursor < source.size()) {
    if (source[cursor] == ']') {
      return true;
    }

    if (source[cursor] != '"') {
      return false;
    }
    ++cursor;

    std::string parsed;
    while (cursor < source.size()) {
      const char current = source[cursor++];
      if (current == '\\') {
        if (cursor >= source.size()) {
          return false;
        }
        parsed.push_back(source[cursor++]);
        continue;
      }
      if (current == '"') {
        break;
      }
      parsed.push_back(current);
    }
    values.push_back(parsed);

    if (cursor >= source.size()) {
      return false;
    }
    if (source[cursor] == ',') {
      ++cursor;
      continue;
    }
    if (source[cursor] == ']') {
      return true;
    }
    return false;
  }

  return false;
}

bool parse_config(const std::string& config_json,
                  BackendConfig& config,
                  std::string& error_message) {
  if (!extract_quoted_value(config_json, "profile", config.profile)) {
    error_message = "config_json is missing a valid `profile` string";
    return false;
  }

  if (!extract_bool_value(config_json, "frame_level", config.frame_level)) {
    error_message = "config_json is missing a valid `features.frame_level` boolean";
    return false;
  }

  if (!extract_string_array(config_json, "enabled", config.enabled_features)) {
    error_message = "config_json is missing a valid `features.enabled` string array";
    return false;
  }

  if (!extract_string_array(config_json, "statistics", config.statistics)) {
    error_message = "config_json is missing a valid `aggregation.statistics` string array";
    return false;
  }

  return true;
}

template <typename T>
std::optional<T> pool_value(const Pool& pool, const std::string& key) {
  if (!pool.contains<T>(key)) {
    return std::nullopt;
  }
  return pool.value<T>(key);
}

void insert_scalar(FamilyMap& features,
                   FamilyMap& aggregation,
                   const std::string& feature,
                   double value) {
  features[feature] = json_number(value);
  aggregation[feature] = "{\"mean\":" + json_number(value) + "}";
}

void insert_vector(FamilyMap& features,
                   FamilyMap& aggregation,
                   const std::string& feature,
                   const std::vector<Real>& values) {
  const std::string array = json_array(values);
  features[feature] = array;
  aggregation[feature] = "{\"mean\":" + array + "}";
}

void insert_frame_scalar(NestedMap& frame_level,
                         const std::string& family,
                         const std::string& feature,
                         const std::vector<Real>& values) {
  frame_level[family][feature] = json_array(values);
}

void insert_frame_vector(NestedMap& frame_level,
                         const std::string& family,
                         const std::string& feature,
                         const std::vector<std::vector<Real>>& values) {
  frame_level[family][feature] = json_array(values);
}

std::vector<Real> fold_hpcp_to_chroma(const std::vector<Real>& hpcp) {
  if (hpcp.empty()) {
    return {};
  }

  const std::size_t bins_per_pitch_class = hpcp.size() / 12;
  if (bins_per_pitch_class == 0 || (hpcp.size() % 12) != 0) {
    return {};
  }

  std::vector<Real> chroma(12, 0.0f);
  for (std::size_t pitch_class = 0; pitch_class < 12; ++pitch_class) {
    Real sum = 0.0f;
    for (std::size_t offset = 0; offset < bins_per_pitch_class; ++offset) {
      sum += hpcp[pitch_class * bins_per_pitch_class + offset];
    }
    chroma[pitch_class] = sum / static_cast<Real>(bins_per_pitch_class);
  }
  return chroma;
}

std::vector<std::vector<Real>> fold_hpcp_frames_to_chroma(
    const std::vector<std::vector<Real>>& hpcp_frames) {
  std::vector<std::vector<Real>> chroma_frames;
  chroma_frames.reserve(hpcp_frames.size());

  for (const auto& frame : hpcp_frames) {
    std::vector<Real> chroma = fold_hpcp_to_chroma(frame);
    if (chroma.empty()) {
      return {};
    }
    chroma_frames.push_back(std::move(chroma));
  }

  return chroma_frames;
}

double clamp_unit_interval(double value) {
  return std::max(0.0, std::min(1.0, value));
}

void add_warning(Payload& payload, const std::string& message) {
  payload.warnings.push_back(message);
}

void add_unavailable_warning(Payload& payload, const std::string& feature) {
  add_warning(payload,
              "requested feature `" + feature +
                  "` is not available from the current Essentia-backed extractor");
}

void map_supported_feature(const Pool& results,
                           const Pool& frame_results,
                           const BackendConfig& config,
                           const std::string& feature,
                           Payload& payload) {
  auto& spectral_features = payload.features["spectral"];
  auto& temporal_features = payload.features["temporal"];
  auto& rhythm_features = payload.features["rhythm"];
  auto& tonal_features = payload.features["tonal"];
  auto& dynamics_features = payload.features["dynamics"];
  auto& metadata_features = payload.features["metadata"];
  auto& spectral = payload.aggregation["spectral"];
  auto& temporal = payload.aggregation["temporal"];
  auto& rhythm = payload.aggregation["rhythm"];
  auto& tonal = payload.aggregation["tonal"];
  auto& dynamics = payload.aggregation["dynamics"];
  auto& metadata = payload.aggregation["metadata"];
  auto& frame_level = payload.frame_level;

  if (feature == "centroid") {
    if (const auto value = pool_value<Real>(results, "lowlevel.spectral_centroid.mean")) {
      insert_scalar(spectral_features, spectral, feature, *value);
      if (config.frame_level) {
        if (const auto frames =
                pool_value<std::vector<Real>>(frame_results, "lowlevel.spectral_centroid")) {
          insert_frame_scalar(frame_level, "spectral", feature, *frames);
        }
      }
      return;
    }
  } else if (feature == "spread") {
    if (const auto value = pool_value<Real>(results, "lowlevel.spectral_spread.mean")) {
      insert_scalar(spectral_features, spectral, feature, *value);
      if (config.frame_level) {
        if (const auto frames =
                pool_value<std::vector<Real>>(frame_results, "lowlevel.spectral_spread")) {
          insert_frame_scalar(frame_level, "spectral", feature, *frames);
        }
      }
      return;
    }
  } else if (feature == "skewness") {
    if (const auto value = pool_value<Real>(results, "lowlevel.spectral_skewness.mean")) {
      insert_scalar(spectral_features, spectral, feature, *value);
      if (config.frame_level) {
        if (const auto frames =
                pool_value<std::vector<Real>>(frame_results, "lowlevel.spectral_skewness")) {
          insert_frame_scalar(frame_level, "spectral", feature, *frames);
        }
      }
      return;
    }
  } else if (feature == "kurtosis") {
    if (const auto value = pool_value<Real>(results, "lowlevel.spectral_kurtosis.mean")) {
      insert_scalar(spectral_features, spectral, feature, *value);
      if (config.frame_level) {
        if (const auto frames =
                pool_value<std::vector<Real>>(frame_results, "lowlevel.spectral_kurtosis")) {
          insert_frame_scalar(frame_level, "spectral", feature, *frames);
        }
      }
      return;
    }
  } else if (feature == "rolloff") {
    if (const auto value = pool_value<Real>(results, "lowlevel.spectral_rolloff.mean")) {
      insert_scalar(spectral_features, spectral, feature, *value);
      if (config.frame_level) {
        if (const auto frames =
                pool_value<std::vector<Real>>(frame_results, "lowlevel.spectral_rolloff")) {
          insert_frame_scalar(frame_level, "spectral", feature, *frames);
        }
      }
      return;
    }
  } else if (feature == "flux") {
    if (const auto value = pool_value<Real>(results, "lowlevel.spectral_flux.mean")) {
      insert_scalar(spectral_features, spectral, feature, *value);
      if (config.frame_level) {
        if (const auto frames =
                pool_value<std::vector<Real>>(frame_results, "lowlevel.spectral_flux")) {
          insert_frame_scalar(frame_level, "spectral", feature, *frames);
        }
      }
      return;
    }
  } else if (feature == "energy") {
    if (const auto value = pool_value<Real>(results, "lowlevel.spectral_energy.mean")) {
      insert_scalar(spectral_features, spectral, feature, *value);
      if (config.frame_level) {
        if (const auto frames =
                pool_value<std::vector<Real>>(frame_results, "lowlevel.spectral_energy")) {
          insert_frame_scalar(frame_level, "spectral", feature, *frames);
        }
      }
      return;
    }
  } else if (feature == "entropy") {
    if (const auto value = pool_value<Real>(results, "lowlevel.spectral_entropy.mean")) {
      insert_scalar(spectral_features, spectral, feature, *value);
      if (config.frame_level) {
        if (const auto frames =
                pool_value<std::vector<Real>>(frame_results, "lowlevel.spectral_entropy")) {
          insert_frame_scalar(frame_level, "spectral", feature, *frames);
        }
      }
      return;
    }
  } else if (feature == "complexity") {
    if (const auto value = pool_value<Real>(results, "lowlevel.spectral_complexity.mean")) {
      insert_scalar(spectral_features, spectral, feature, *value);
      if (config.frame_level) {
        if (const auto frames =
                pool_value<std::vector<Real>>(frame_results, "lowlevel.spectral_complexity")) {
          insert_frame_scalar(frame_level, "spectral", feature, *frames);
        }
      }
      return;
    }
  } else if (feature == "hfc") {
    if (const auto value = pool_value<Real>(results, "lowlevel.hfc.mean")) {
      insert_scalar(spectral_features, spectral, feature, *value);
      if (config.frame_level) {
        if (const auto frames = pool_value<std::vector<Real>>(frame_results, "lowlevel.hfc")) {
          insert_frame_scalar(frame_level, "spectral", feature, *frames);
        }
      }
      return;
    }
  } else if (feature == "strong_peak") {
    if (const auto value = pool_value<Real>(results, "lowlevel.spectral_strongpeak.mean")) {
      insert_scalar(spectral_features, spectral, feature, *value);
      if (config.frame_level) {
        if (const auto frames =
                pool_value<std::vector<Real>>(frame_results, "lowlevel.spectral_strongpeak")) {
          insert_frame_scalar(frame_level, "spectral", feature, *frames);
        }
      }
      return;
    }
  } else if (feature == "dissonance") {
    if (const auto value = pool_value<Real>(results, "lowlevel.dissonance.mean")) {
      insert_scalar(spectral_features, spectral, feature, *value);
      if (config.frame_level) {
        if (const auto frames =
                pool_value<std::vector<Real>>(frame_results, "lowlevel.dissonance")) {
          insert_frame_scalar(frame_level, "spectral", feature, *frames);
        }
      }
      return;
    }
  } else if (feature == "mfcc") {
    if (const auto value = pool_value<std::vector<Real>>(results, "lowlevel.mfcc.mean")) {
      insert_vector(spectral_features, spectral, feature, *value);
      if (config.frame_level) {
        if (const auto frames =
                pool_value<std::vector<std::vector<Real>>>(frame_results, "lowlevel.mfcc")) {
          insert_frame_vector(frame_level, "spectral", feature, *frames);
        }
      }
      return;
    }
  } else if (feature == "bark_bands") {
    if (const auto value = pool_value<std::vector<Real>>(results, "lowlevel.barkbands.mean")) {
      insert_vector(spectral_features, spectral, feature, *value);
      if (config.frame_level) {
        if (const auto frames =
                pool_value<std::vector<std::vector<Real>>>(frame_results, "lowlevel.barkbands")) {
          insert_frame_vector(frame_level, "spectral", feature, *frames);
        }
      }
      return;
    }
  } else if (feature == "mel_bands") {
    if (const auto value = pool_value<std::vector<Real>>(results, "lowlevel.melbands.mean")) {
      insert_vector(spectral_features, spectral, feature, *value);
      if (config.frame_level) {
        if (const auto frames =
                pool_value<std::vector<std::vector<Real>>>(frame_results, "lowlevel.melbands")) {
          insert_frame_vector(frame_level, "spectral", feature, *frames);
        }
      }
      return;
    }
  } else if (feature == "erb_bands") {
    if (const auto value = pool_value<std::vector<Real>>(results, "lowlevel.erbbands.mean")) {
      insert_vector(spectral_features, spectral, feature, *value);
      if (config.frame_level) {
        if (const auto frames =
                pool_value<std::vector<std::vector<Real>>>(frame_results, "lowlevel.erbbands")) {
          insert_frame_vector(frame_level, "spectral", feature, *frames);
        }
      }
      return;
    }
  } else if (feature == "gfcc") {
    if (const auto value = pool_value<std::vector<Real>>(results, "lowlevel.gfcc.mean")) {
      insert_vector(spectral_features, spectral, feature, *value);
      if (config.frame_level) {
        if (const auto frames =
                pool_value<std::vector<std::vector<Real>>>(frame_results, "lowlevel.gfcc")) {
          insert_frame_vector(frame_level, "spectral", feature, *frames);
        }
      }
      return;
    }
  } else if (feature == "zcr") {
    if (const auto value = pool_value<Real>(results, "lowlevel.zerocrossingrate.mean")) {
      insert_scalar(temporal_features, temporal, feature, *value);
      if (config.frame_level) {
        if (const auto frames =
                pool_value<std::vector<Real>>(frame_results, "lowlevel.zerocrossingrate")) {
          insert_frame_scalar(frame_level, "temporal", feature, *frames);
        }
      }
      return;
    }
  } else if (feature == "rms") {
    if (const auto value = pool_value<Real>(results, "lowlevel.spectral_rms.mean")) {
      insert_scalar(temporal_features, temporal, feature, *value);
      if (config.frame_level) {
        if (const auto frames =
                pool_value<std::vector<Real>>(frame_results, "lowlevel.spectral_rms")) {
          insert_frame_scalar(frame_level, "temporal", feature, *frames);
        }
      }
      return;
    }
  } else if (feature == "dynamic_range") {
    if (const auto value = pool_value<Real>(results, "lowlevel.loudness_ebu128.loudness_range")) {
      insert_scalar(temporal_features, temporal, feature, *value);
      return;
    }
  } else if (feature == "onset_rate") {
    if (const auto value = pool_value<Real>(results, "rhythm.onset_rate")) {
      insert_scalar(rhythm_features, rhythm, feature, *value);
      return;
    }
  } else if (feature == "tempo") {
    if (const auto value = pool_value<Real>(results, "rhythm.bpm")) {
      insert_scalar(rhythm_features, rhythm, feature, *value);
      return;
    }
  } else if (feature == "beat_period") {
    if (const auto value = pool_value<Real>(results, "rhythm.bpm")) {
      if (*value > 0.0f) {
        insert_scalar(rhythm_features, rhythm, feature, 60.0 / static_cast<double>(*value));
        return;
      }
    }
  } else if (feature == "hpcp") {
    if (const auto value = pool_value<std::vector<Real>>(results, "tonal.hpcp.mean")) {
      insert_vector(tonal_features, tonal, feature, *value);
      if (config.frame_level) {
        if (const auto frames =
                pool_value<std::vector<std::vector<Real>>>(frame_results, "tonal.hpcp")) {
          insert_frame_vector(frame_level, "tonal", feature, *frames);
        }
      }
      return;
    }
  } else if (feature == "chroma") {
    if (const auto value = pool_value<std::vector<Real>>(results, "tonal.hpcp.mean")) {
      const std::vector<Real> chroma = fold_hpcp_to_chroma(*value);
      if (!chroma.empty()) {
        insert_vector(tonal_features, tonal, feature, chroma);
        if (config.frame_level) {
          if (const auto frames =
                  pool_value<std::vector<std::vector<Real>>>(frame_results, "tonal.hpcp")) {
            std::vector<std::vector<Real>> chroma_frames = fold_hpcp_frames_to_chroma(*frames);
            if (!chroma_frames.empty()) {
              insert_frame_vector(frame_level, "tonal", feature, chroma_frames);
            }
          }
        }
        return;
      }
    }
  } else if (feature == "key_strength") {
    if (const auto value = pool_value<Real>(results, "tonal.key_edma.strength")) {
      insert_scalar(tonal_features, tonal, feature, *value);
      return;
    }
  } else if (feature == "tuning_frequency") {
    if (const auto value = pool_value<Real>(results, "tonal.tuning_frequency")) {
      insert_scalar(tonal_features, tonal, feature, *value);
      return;
    }
  } else if (feature == "loudness") {
    if (const auto value = pool_value<Real>(results, "lowlevel.average_loudness")) {
      insert_scalar(dynamics_features, dynamics, feature, *value);
      return;
    }
  } else if (feature == "loudness_ebu") {
    if (const auto value = pool_value<Real>(results, "lowlevel.loudness_ebu128.integrated")) {
      insert_scalar(dynamics_features, dynamics, feature, *value);
      if (config.frame_level) {
        if (const auto frames =
                pool_value<std::vector<Real>>(frame_results, "lowlevel.loudness_ebu128.momentary")) {
          insert_frame_scalar(frame_level, "dynamics", feature, *frames);
        }
      }
      return;
    }
  } else if (feature == "dynamic_complexity") {
    if (const auto value = pool_value<Real>(results, "lowlevel.dynamic_complexity")) {
      insert_scalar(dynamics_features, dynamics, feature, *value);
      return;
    }
  } else if (feature == "duration") {
    if (const auto value = pool_value<Real>(results, "metadata.audio_properties.length")) {
      insert_scalar(metadata_features, metadata, feature, *value);
      return;
    }
  } else if (feature == "silence_ratio") {
    if (const auto value = pool_value<Real>(results, "lowlevel.silence_rate_60dB.mean")) {
      insert_scalar(metadata_features, metadata, feature, clamp_unit_interval(*value));
      return;
    }
  } else if (feature == "active_ratio") {
    if (const auto value = pool_value<Real>(results, "lowlevel.silence_rate_60dB.mean")) {
      insert_scalar(metadata_features, metadata, feature, 1.0 - clamp_unit_interval(*value));
      return;
    }
  }

  add_unavailable_warning(payload, feature);
}

std::map<std::string, std::string> build_audio_block(const Pool& results) {
  std::map<std::string, std::string> fields;

  if (const auto value = pool_value<Real>(results, "metadata.audio_properties.sample_rate")) {
    fields.emplace("sample_rate", json_number(*value));
  }
  if (const auto value =
          pool_value<Real>(results, "metadata.audio_properties.number_channels")) {
    fields.emplace("channels", json_number(*value));
  }
  if (const auto value = pool_value<Real>(results, "metadata.audio_properties.length")) {
    fields.emplace("duration_seconds", json_number(*value));
  }

  return fields;
}

std::string analyze_file_impl(const std::string& path, const BackendConfig& config) {
  for (const std::string& statistic : config.statistics) {
    if (statistic != "mean") {
      return error_payload("unsupported_statistic",
                           "the native Essentia backend currently supports only `mean` aggregation");
    }
  }

  ensure_essentia_initialized();

  essentia::standard::Algorithm* extractor = nullptr;
  try {
    extractor = essentia::standard::AlgorithmFactory::create(
        "MusicExtractor", "analysisSampleRate", 44100, "lowlevelStats",
        std::vector<std::string>{"mean"}, "rhythmStats", std::vector<std::string>{"mean"},
        "tonalStats", std::vector<std::string>{"mean"}, "mfccStats",
        std::vector<std::string>{"mean"}, "gfccStats", std::vector<std::string>{"mean"});

    Pool results;
    Pool frame_results;
    extractor->input("filename").set(path);
    extractor->output("results").set(results);
    extractor->output("resultsFrames").set(frame_results);
    extractor->compute();

    Payload payload;
    payload.frame_level_enabled = config.frame_level;
    for (const std::string& feature : config.enabled_features) {
      map_supported_feature(results, frame_results, config, feature, payload);
    }

    delete extractor;
    extractor = nullptr;
    return render_payload(build_audio_block(results), payload);
  } catch (const std::exception& error) {
    if (extractor != nullptr) {
      delete extractor;
    }
    return error_payload("analysis_error", error.what());
  }
}

}  // namespace

char* afl_essentia_backend_version(void) {
  ensure_essentia_initialized();

  std::ostringstream version;
  version << "essentia " << essentia::version << " (" << essentia::version_git_sha
          << "), music_extractor " << MUSIC_EXTRACTOR_VERSION;
  return duplicate_string(version.str());
}

char* afl_essentia_analyze_file(const char* path, const char* config_json) {
  if (path == nullptr || config_json == nullptr) {
    return duplicate_string(
        error_payload("invalid_input", "path and config_json must be non-null"));
  }

  BackendConfig config;
  std::string parse_error;
  if (!parse_config(config_json, config, parse_error)) {
    return duplicate_string(error_payload("invalid_config", parse_error));
  }

  return duplicate_string(analyze_file_impl(path, config));
}

void afl_essentia_free_string(char* value) {
  delete[] value;
}
