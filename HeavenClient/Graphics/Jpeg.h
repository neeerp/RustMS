#pragma once

#include <cstdint>
#include <vector>

namespace ms
{
	// Decode the JPEG `jpeg` into BGRA `pixels`, as the atlas takes them.
	// False, leaving the outputs alone, if it isn't one.
	bool decode_jpeg(const std::vector<uint8_t> &jpeg, int16_t &width, int16_t &height, std::vector<uint8_t> &pixels);
}
