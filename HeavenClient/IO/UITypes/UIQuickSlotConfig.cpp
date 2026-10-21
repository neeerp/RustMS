#include "UIQuickSlotConfig.h"

#include "UIKeyConfig.h"
#include "UINotice.h"

#include "../Keyboard.h"
#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#ifdef _WIN32
#include <glfw3.h>
#else
#include <GLFW/glfw3.h>
#endif

namespace ms
{
	namespace
	{
		constexpr int16_t WIDTH = 266, HEIGHT = 238;
		const Point<int16_t> OK_POS(157, 208), CANCEL_POS(210, 208);
		// A click inside the window but off the cells leaves them
		// (FUN_0072d0b8).
		const Rectangle<int16_t> GRID(Point<int16_t>(65, 113), Point<int16_t>(202, 177));
		// A cell's label sits just inside it.
		const Point<int16_t> LABEL(2, 2);

		// FUN_0072df19.
		Point<int16_t> cell_pos(size_t cell)
		{
			return Point<int16_t>(65 + 35 * static_cast<int16_t>(cell % 4), cell < 4 ? 113 : 147);
		}
	}

	UIQuickSlotConfig::UIQuickSlotConfig() : UIElement(Point<int16_t>(), Point<int16_t>(WIDTH, HEIGHT)),
		working(UI::get().get_quick_slots().keys())
	{
		nl::node keyconfig = NxFiles::ui_classic["UIWindow.img"]["KeyConfig"];
		nl::node src = keyconfig["quickslotConfig"];

		sprites.emplace_back(src["backgrnd"]);

		buttons[Buttons::OK] = std::make_unique<MapleButton>(keyconfig["BtOK"], OK_POS);
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(keyconfig["BtCancel"], CANCEL_POS);

		for (size_t i = 0; i < working.size(); i++)
			buttons[Buttons::CELL + i] = std::make_unique<MapleButton>(src["BtQuickSetting"], cell_pos(i));

		for (nl::node label : src["key"])
			labels[std::stoi(label.name())] = label;

		update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());
	}

	// Each cell shows, filling it, what Key Config has on its key, and the
	// key's label just inside it (FUN_0072d162). The icons go on the window
	// and the labels on a layer one above it, made after the cells, so the
	// selected cell's fill covers the icon but not the label.
	void UIQuickSlotConfig::draw(float inter) const
	{
		UIElement::draw_sprites(inter);

		if (auto keyconfig = UI::get().get_element<UIKeyConfig>())
			for (size_t i = 0; i < working.size(); i++)
				keyconfig->key_icon(working[i]).draw(position + cell_pos(i));

		UIElement::draw_buttons(inter);

		for (size_t i = 0; i < working.size(); i++)
		{
			auto label = labels.find(working[i]);

			if (label != labels.end())
				label->second.draw(position + cell_pos(i) + LABEL);
		}
	}

	// v83 centres it on its 800 x 600 screen.
	void UIQuickSlotConfig::update_screen(int16_t new_width, int16_t new_height)
	{
		position = Point<int16_t>((new_width - WIDTH) / 2, (new_height - HEIGHT) / 2);
	}

	Cursor::State UIQuickSlotConfig::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Point<int16_t> local = cursorpos - position;

		if (mouse_down && !clicked && Rectangle<int16_t>(Point<int16_t>(), dimension).contains(local)
			&& !GRID.contains(local))
			unfocus();

		mouse_down = clicked;

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIQuickSlotConfig::send_raw_key(int32_t keycode, bool pressed)
	{
		if (!pressed)
			return;

		switch (keycode)
		{
			// Tab goes from nowhere or the buttons to the first cell, and
			// from the cells to OK (FUN_0072ddb3).
			case GLFW_KEY_TAB:
				if (focus == Focus::CELLS)
					focus_button(false);
				else
					select(0);

				break;
			case GLFW_KEY_ENTER:
			case GLFW_KEY_KP_ENTER:
				ok();
				break;
			case GLFW_KEY_ESCAPE:
				if (focus == Focus::NONE)
				{
					deactivate();
				}
				else
				{
					unfocus();
					selected = 0;
				}

				break;
			// The arrows stay inside a row (FUN_0072de13) and step between
			// OK and Cancel (FUN_0072de7b). At a row's end they do nothing
			// (72cf84, 72cf23).
			case GLFW_KEY_LEFT:
				if (focus == Focus::CELLS)
				{
					if (selected % 4 != 0)
						select(selected - 1);
				}
				else if (focus == Focus::BUTTONS)
				{
					focus_button(false);
				}

				break;
			case GLFW_KEY_RIGHT:
				if (focus == Focus::CELLS)
				{
					if (selected % 4 != 3)
						select(selected + 1);
				}
				else if (focus == Focus::BUTTONS)
				{
					focus_button(true);
				}

				break;
			case GLFW_KEY_UP:
				if (focus == Focus::CELLS && selected >= 4)
					select(selected - 4);

				break;
			case GLFW_KEY_DOWN:
				if (focus == Focus::CELLS && selected < 4)
					select(selected + 4);

				break;
			default:
				if (focus == Focus::CELLS)
					press_key(keycode);

				break;
		}
	}

	UIElement::Type UIQuickSlotConfig::get_type() const
	{
		return TYPE;
	}

	Button::State UIQuickSlotConfig::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::OK:
				ok();
				break;
			case Buttons::CANCEL:
				deactivate();
				break;
			default:
				select(buttonid - Buttons::CELL);
				break;
		}

		return Button::State::NORMAL;
	}

	void UIQuickSlotConfig::select(size_t cell)
	{
		clear_highlights();
		focus = Focus::CELLS;
		selected = cell;
		buttons[Buttons::CELL + cell]->set_focused(true);
	}

	void UIQuickSlotConfig::focus_button(bool cancel)
	{
		clear_highlights();
		focus = Focus::BUTTONS;
		buttons[cancel ? Buttons::CANCEL : Buttons::OK]->set_focused(true);
	}

	// The selected cell is kept for when the cells are clicked back into.
	void UIQuickSlotConfig::unfocus()
	{
		clear_highlights();
		focus = Focus::NONE;
	}

	void UIQuickSlotConfig::clear_highlights()
	{
		for (auto& button : buttons)
			button.second->set_focused(false);
	}

	// The selection stays on the cell (FUN_0072df55).
	void UIQuickSlotConfig::press_key(int32_t keycode)
	{
		switch (QuickSlots::assign(working, selected, Keyboard::quickslot_key(keycode)))
		{
			case QuickSlots::Assign::DISABLED:
				UI::get().emplace<UIOk>("The key you just pressed is a key that has been disabled.", [](bool) {});
				break;
			case QuickSlots::Assign::IN_USE:
				UI::get().emplace<UIOk>("This key is already being used.", [](bool) {});
				break;
			default:
				break;
		}
	}

	// The status bar shows the new keys at once; Key Config sends them.
	void UIQuickSlotConfig::ok()
	{
		UI::get().get_quick_slots().set(working);
		deactivate();
	}
}
