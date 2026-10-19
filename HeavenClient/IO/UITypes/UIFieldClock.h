#pragma once

#include "../UIElement.h"

#include "../../Timer.h"

#include <cstdint>

namespace ms
{
	// The countdown at the top of the screen (v83 0x52c00e, drawn by 0x49a0f4). It counts while running and
	// keeps its reading, from 0 shown as 0, until taken down; the server moves the player on.
	class UIClockWindow : public UIElement
	{
	public:
		void draw(float alpha) const override;

		void update() override;

		void update_screen(int16_t new_width, int16_t new_height) override;

		// Clicks fall through to the map.
		bool is_in_range(Point<int16_t> cursorpos) const override;

		// Count down from the reading (0x49aeee). A running clock carries on.
		void start();

		bool is_running() const;

	protected:
		explicit UIClockWindow(int32_t seconds);

		int32_t get_reading() const;

	private:
		int32_t reading;
		bool running;
		ContinuousTimer::point started;
		int32_t from;
	};

	// The field's clock (CLOCK kinds 2 and 3), which goes with the field and STOP_CLOCK takes down.
	class UIFieldClock : public UIClockWindow
	{
	public:
		static constexpr Type TYPE = UIElement::Type::FIELDCLOCK;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		// `switched` for a kind 3 clock, which the next kind 3 takes down.
		UIFieldClock(int32_t seconds, bool switched);

		bool is_switched() const;

		// Another kind of CLOCK leaves this one to the next kind 3.
		void unswitch();

		UIElement::Type get_type() const override;

	private:
		bool switched;
	};

	// An event's clock (CLOCK kind 0), which stays up across the event's maps until it reads 0 (0xa24c82,
	// 0xa03350).
	class UIEventClock : public UIClockWindow
	{
	public:
		static constexpr Type TYPE = UIElement::Type::EVENTCLOCK;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		explicit UIEventClock(int32_t seconds);

		void update() override;

		UIElement::Type get_type() const override;
	};
}
