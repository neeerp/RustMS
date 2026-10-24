#pragma once

#include "../UIElement.h"

#include "../../Audio/Audio.h"
#include "../../Character/TimedQuests.h"
#include "../../Graphics/Texture.h"

#include <cstdint>
#include <vector>

namespace ms
{
	// The clocks of timed quests (v83 FUN_0088e2f5 with its alarm clock FUN_0088d2ce): a 52x12 bar whose 48-step
	// gauge empties as the time runs out, and to its right a clock whose frames spread over the whole time. A
	// quest whose timerUI is "default" rings its alarm for the last stretch, and booms when the clock is taken
	// down. Hovering over a bar shows the time left. The clocks themselves are timed_quests::shown().
	class UITimedQuests : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::TIMEDQUESTS;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UITimedQuests();
		~UITimedQuests() override;

		void draw(float alpha) const override;

		void update() override;

		void update_screen(int16_t new_width, int16_t new_height) override;

		bool is_in_range(Point<int16_t> cursorpos) const override;
		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;
		void remove_cursor() override;

		UIElement::Type get_type() const override;

		// A quest's clock is taken down, by mode 7 or as it runs out.
		void taken_down(int16_t quest);

	private:
		// The clock of one of shown()'s clocks: its ring frames and its sound, looked up once.
		struct Alarm
		{
			uint32_t serial;
			Sound loop;
			bool alarmed;
			bool ringing;
			std::vector<Texture> faces;
		};

		// The clock under the cursor, if any.
		const timed_quests::Clock* clock_at(Point<int16_t> cursorpos) const;

		const Alarm* find_alarm(uint32_t serial) const;
		Alarm& alarm_for(const timed_quests::Clock& clock);

		// v83 draws in an 800 px wide screen.
		Point<int16_t> origin() const;

		Texture backgrnd;
		Texture gauge[3];

		std::vector<Alarm> alarms;
		uint32_t hovered;
		int32_t since_tooltip;
	};
}
