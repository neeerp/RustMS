#include "Jpeg.h"

#include "stb_image.h"

#include <utility>

namespace ms
{
	bool decode_jpeg(const std::vector<uint8_t> &jpeg, int16_t &width, int16_t &height, std::vector<uint8_t> &pixels)
	{
		if (jpeg.empty())
			return false;

		int w = 0;
		int h = 0;
		int channels = 0;
		stbi_uc *rgba = stbi_load_from_memory(jpeg.data(), static_cast<int>(jpeg.size()), &w, &h, &channels, 4);

		if (!rgba)
			return false;

		if (w <= 0 || h <= 0 || w > INT16_MAX || h > INT16_MAX)
		{
			stbi_image_free(rgba);
			return false;
		}

		pixels.assign(rgba, rgba + static_cast<size_t>(w) * h * 4);
		stbi_image_free(rgba);

		for (size_t i = 0; i < pixels.size(); i += 4)
			std::swap(pixels[i], pixels[i + 2]);

		width = static_cast<int16_t>(w);
		height = static_cast<int16_t>(h);

		return true;
	}
}
