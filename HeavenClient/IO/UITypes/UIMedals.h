#pragma once

#include <cstdint>
#include <vector>

#include "../UIDragElement.h"

#include "../Components/Slider.h"

#include "../../Character/MedalList.h"
#include "../../Character/QuestLog.h"
#include "../../Graphics/Animation.h"

namespace ms
{
	// v83's title window (UI 0x1f, UIClassic.nx UIWindow.img/Title): the
	// titles, the quests in QuestInfo.img's area 51, by tab, and beside the
	// list the selected one's details (report b147).
	class UIMedals : public UIDragElement<PosMEDALS>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::MEDALS;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = true;

		UIMedals(const QuestLog& quests);

		void draw(float inter) const override;
		void update() override;
		void toggle_active() override;
		void update_screen(int16_t width, int16_t height) override;
		void send_scroll(double yoffset) override;
		void send_key(int32_t keycode, bool pressed, bool escape) override;
		Cursor::State send_cursor(bool clicking, Point<int16_t> cursorpos) override;
		void remove_cursor() override;

		// Open title `id`'s tab and select it there, if listed.
		void show_title(int16_t id);

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		enum Buttons : uint16_t
		{
			TAB0,
			HIDE = TAB0 + medal_list::NUM_TABS,
			CHALLENGE,
			FORFEIT
		};

		struct Line
		{
			medal_list::Row row;
			Text name, requirement;
			// The line under the name, before it is cut to fit.
			std::string second;
			int32_t medal;
			// Where the row starts, from the window's top.
			int16_t top;
		};

		void change_tab(uint8_t tabid);
		void refresh_list();
		void layout();
		void select(int16_t id);
		void write_details();
		void show_details(bool open);
		void challenge();
		void give_up();
		int16_t max_offset() const;
		const Line* line_at(Point<int16_t> cursor) const;
		void clear_tooltip();

		const QuestLog& questlog;
		uint8_t tab = medal_list::BASIC;
		uint8_t folded[medal_list::NUM_TABS] = {};
		std::vector<medal_list::Medal> medals;
		std::vector<Line> lines;
		int16_t offset = 0;
		int16_t obtained = 0;
		Text count;

		Texture background, strip, row, frame;
		Animation worn_frame;
		Texture tabs_enabled[medal_list::NUM_TABS], tabs_disabled[medal_list::NUM_TABS];
		Texture fold_boxes[2];
		Slider slider;

		// The details.
		Texture detail_background, portrait;
		Slider detail_slider;
		bool details_open = false;
		int16_t selected = -1, detail_offset = 0;
		medal_list::State selected_state = medal_list::OBTAINABLE;
		Text title, requirement, description;
		Point<int16_t> cursor_relative;
		bool tooltip = false;

		uint64_t seen_revision = 0;
		int16_t seen_job = -1, seen_level = -1;
		int32_t worn = 0;
	};
}
