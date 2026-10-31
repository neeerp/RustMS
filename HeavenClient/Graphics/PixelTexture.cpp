#include "PixelTexture.h"

#include "GraphicsGL.h"
#include "Jpeg.h"

namespace ms
{
	namespace
	{
		// Ids above any a file's bitmap takes, which are addresses.
		size_t next_id()
		{
			static size_t last = size_t(1) << 62;

			return ++last;
		}
	}

	PixelTexture::PixelTexture() : id(0)
	{}

	PixelTexture PixelTexture::from_jpeg(const std::vector<uint8_t> &jpeg)
	{
		PixelTexture texture;
		int16_t width = 0;
		int16_t height = 0;
		auto decoded = std::make_shared<std::vector<uint8_t>>();

		if (!decode_jpeg(jpeg, width, height, *decoded))
			return texture;

		texture.id = next_id();
		texture.dimensions = Point<int16_t>(width, height);
		texture.pixels = decoded;

		return texture;
	}

	void PixelTexture::draw(const DrawArgument &args) const
	{
		if (!is_valid())
			return;

		GraphicsGL::get().draw(
				id,
				dimensions.x(),
				dimensions.y(),
				pixels->data(),
				args.get_rectangle(Point<int16_t>(), dimensions),
				args.get_color(),
				args.get_angle()
		);
	}

	bool PixelTexture::is_valid() const
	{
		return id != 0 && pixels;
	}

	Point<int16_t> PixelTexture::get_dimensions() const
	{
		return dimensions;
	}
}
