#pragma once

#include <cstdint>
#include <cstddef>
#include <cstring>

enum class Tag {
  Invalid = 0,
  Info = 1,
  Battery = 2,
  Haptic = 3,
};

struct CommandServer {
  Tag tag;

  union {
    struct { 
      uint8_t channel; 
      uint8_t strength; 
      uint16_t duration; 
    } haptic;
  };

  static CommandServer from_bytes(uint8_t bytes[], int len) {
    if (len < 2) {
      return CommandServer { .tag = Tag::Invalid };
    }

    switch ((Tag)bytes[0]) {
      case Tag::Haptic: return CommandServer {
        .tag = Tag::Haptic,
        .haptic = {
          .channel = bytes[2],
          .strength = bytes[3],
          .duration = (uint16_t)(((uint16_t)bytes[5] << 8) | bytes[4]),
        }
      };
    }

    return CommandServer { .tag = Tag::Invalid };
  }
};

struct CommandClient {
  Tag tag;

  union {
    struct { uint8_t channel; char name[32]; } info;
    struct { uint8_t level; } battery;
    struct { uint8_t status; } haptic;
  };

  size_t to_bytes(uint8_t bytes[]) {
    bytes[0] = (int)tag;
    bytes[1] = 2;

    switch (tag) {
      case Tag::Info: {
        bytes[2] = info.channel;
        int len = strlen(info.name);
        bytes[1] = 3 + len;
        memcpy(bytes + 3, info.name, len);
        return 3 + len;
      }
      case Tag::Battery: {
        bytes[1] = 3;
        bytes[2] = battery.level;
        return 3;
      }
      case Tag::Haptic: {
        bytes[1] = 3;
        bytes[2] = haptic.status;
        return 3;
      }
      case Tag::Invalid: {
        return 2;
      }
    }
    return 0;
  }
};