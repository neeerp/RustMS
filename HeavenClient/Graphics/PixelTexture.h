#pragma once

#include "DrawArgument.h"

#include <cstdint>
#include <memory>
#include <vector>

namespace ms
{
	// An image sent by the server rather than read from a file, such as the
	// Lie Detector's picture.
	class PixelTexture
	{
	public:
		// Invalid, drawing nothing.
		PixelTexture();

		// The picture in `jpeg`; invalid if it doesn't decode.
		static PixelTexture from_jpeg(const std::vector<uint8_t> &jpeg);

		void draw(const DrawArgument &args) const;

		bool is_valid() const;

		Point<int16_t> get_dimensions() const;

	private:
		size_t id;
		Point<int16_t> dimensions;
		std::shared_ptr<const std::vector<uint8_t>> pixels;
	};
}
