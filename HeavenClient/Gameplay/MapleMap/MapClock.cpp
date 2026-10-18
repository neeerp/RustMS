#include "MapClock.h"

#include "../../Constants.h"

#include <nlnx/nx.hpp>

#include <cmath>

namespace ms
{
	MapClock::MapClock(nl::node src)
	{
		nl::node box = src["clock"];

		// v83 makes no clock unless all four are there.
		if (!box["x"] || !box["y"] || !box["width"] || !box["height"])
			return;

		int32_t width = static_cast<int32_t>(box["width"].get_integer());
		int32_t height = static_cast<int32_t>(box["height"].get_integer());

		nl::node font = nl::nx::map["Obj"]["etc.img"]["clock"]["fontTime"];

		for (size_t i = 0; i < digits.size(); i++)
			digits[i] = font[std::to_string(i)];

		colon = font["comma"];
		am = font["am"];
		pm = font["pm"];
		present = true;
		position = Point<int16_t>(box["x"], box["y"]);
		layout = wall_clock::layout(width, height, digits[0].width(), digits[0].height(), colon.width(), am.width());
	}

	MapClock::MapClock() {}

	bool MapClock::is_present() const
	{
		return present;
	}

	void MapClock::set(uint8_t hour, uint8_t minute, uint8_t second)
	{
		clock.set(wall_clock::set(hour, minute, second));
	}

	void MapClock::update()
	{
		if (present)
			clock.update(Constants::TIMESTEP);
	}

	void MapClock::draw(double viewx, double viewy) const
	{
		if (!present)
			return;

		// The glyphs' origins are all at their top left.
		Point<int16_t> box = position + Point<int16_t>(static_cast<int16_t>(std::round(viewx)),
													   static_cast<int16_t>(std::round(viewy)));
		auto at = [&](int32_t x) {
			return box + Point<int16_t>(static_cast<int16_t>(x), static_cast<int16_t>(layout.y));
		};

		const wall_clock::Time &time = clock.get();
		(time.pm ? pm : am).draw(DrawArgument(at(layout.tag)));
		digits[time.hour / 10 % 10].draw(DrawArgument(at(layout.hour_tens)));
		digits[time.hour % 10].draw(DrawArgument(at(layout.hour_ones)));
		colon.draw(DrawArgument(at(layout.colon), wall_clock::colon_opacity(time.second)));
		digits[time.minute / 10 % 10].draw(DrawArgument(at(layout.minute_tens)));
		digits[time.minute % 10].draw(DrawArgument(at(layout.minute_ones)));
	}
}
