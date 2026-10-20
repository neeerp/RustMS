#pragma once

#include "../UIElement.h"

#include "../Components/Slider.h"

#include "../../Character/SpResetRules.h"
#include "../../Graphics/Text.h"

#include <array>
#include <cstdint>
#include <functional>
#include <vector>

namespace ms
{
	// SP Reset's three dialogs in turn (v83 FUN_00a0a63f type 0x18 on UIWindow.img/Reset/SP, report 195): the
	// skill a level leaves, from a tab per advancement up to the item's, the skill of the item's advancement it
	// goes to, then the change to confirm. Picking a row moves on at once; only the last has OK. Closing any of
	// them, or Escape, sends nothing.
	class UISpReset : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::SPRESET;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		static constexpr int16_t ROWS = 4;

		// `onconfirm` gets the skill the level goes to and the one it leaves.
		UISpReset(int32_t item_id, std::function<void(int32_t to, int32_t from)> onconfirm);

		void draw(float alpha) const override;

		void update() override;

		void update_screen(int16_t new_width, int16_t new_height) override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_scroll(double yoffset) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		enum Step
		{
			SOURCE,
			TARGET,
			CONFIRM
		};

		// Each dialog's rows have their own ids, so a click that moves on leaves the next one's button alone.
		enum Buttons : uint16_t
		{
			SOURCE_ROW = 0,
			TARGET_ROW = ROWS,
			CLOSE = 2 * ROWS,
			OK,
			CANCEL
		};

		struct Entry
		{
			int32_t skill_id;
			Text name;
			Text level;
			bool enabled;
		};

		void show(Step step);

		// Lists the skills of the path's `tab`th job, as the skill window does (FUN_008b2c62).
		void list(int32_t tab);

		// Which of the rows shown may be picked (FUN_008b286e), done when the list or its scroll changes.
		void refresh();

		void confirm();

		sp_reset::Character character() const;

		std::function<void(int32_t to, int32_t from)> onconfirm;
		Step step;
		// The item's advancement, as a tab, and the source dialog's tab.
		int32_t item_tab;
		int32_t tab;
		std::vector<int32_t> path;
		std::vector<Entry> entries;
		int16_t first;
		Slider slider;
		bool held;
		int32_t from;
		int32_t to;
		Text spare;
		Text less;
		Text less_by;
		Text more;
		Text more_by;
		Text from_name;
		Text to_name;
	};
}
