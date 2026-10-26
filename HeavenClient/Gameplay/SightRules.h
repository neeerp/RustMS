#pragma once

#include <array>
#include <cstdint>

// The darkness that leaves the player a narrow sight: Fear's (FUN_0053868d, Effect/MapEff.img/Viewrange/3)
// and a LimitedView field's (info/fieldType 9, CField subclass 0xaf3fcc, Viewrange/1). Both lay a black
// layer fixed on the screen and cut the sight bitmap out of it, its top left this far from the player's
// feet whatever its NX origin; outside the sight's square the screen stays black.
namespace ms::sight
{
	constexpr int32_t LIMITED_VIEW = 9;

	struct Offset
	{
		int16_t x;
		int16_t y;
	};

	// Fear's sight (151x176) at feet + (320 - 400, 180 - 300).
	constexpr Offset FEAR = { -80, -120 };
	// A LimitedView's (316x316) at feet + (237 - 400, 110 - 300) (0x55bdef).
	constexpr Offset LIMITED = { -163, -190 };

	// Whether a map with this info/fieldType keeps its players in the dark. Only the field's class
	// does so; no packet turns it on or off.
	inline bool limits_view(int32_t fieldtype)
	{
		return fieldtype == LIMITED_VIEW;
	}

	struct Strip
	{
		int16_t left;
		int16_t top;
		int16_t width;
		int16_t height;
	};

	// The black around a sight of this size with its top left at (left, top) on a screen this big:
	// above, below, and to the left and right of it. A strip the sight pushes off the screen is empty.
	inline std::array<Strip, 4> strips(int16_t left, int16_t top, int16_t width, int16_t height, int16_t screen_width,
									   int16_t screen_height)
	{
		auto clamp = [](int32_t value, int32_t high) {
			return static_cast<int16_t>(value < 0 ? 0 : (value > high ? high : value));
		};

		int16_t top_edge = clamp(top, screen_height);
		int16_t bottom_edge = clamp(top + height, screen_height);
		int16_t left_edge = clamp(left, screen_width);
		int16_t right_edge = clamp(left + width, screen_width);
		int16_t middle = bottom_edge - top_edge;

		return { {
			{ 0, 0, screen_width, top_edge },
			{ 0, bottom_edge, screen_width, static_cast<int16_t>(screen_height - bottom_edge) },
			{ 0, top_edge, left_edge, middle },
			{ right_edge, top_edge, static_cast<int16_t>(screen_width - right_edge), middle },
		} };
	}
}
