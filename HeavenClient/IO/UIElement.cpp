//////////////////////////////////////////////////////////////////////////////////
//	This file is part of the continued Journey MMORPG client					//
//	Copyright (C) 2015-2019  Daniel Allendorf, Ryan Payton						//
//																				//
//	This program is free software: you can redistribute it and/or modify		//
//	it under the terms of the GNU Affero General Public License as published by	//
//	the Free Software Foundation, either version 3 of the License, or			//
//	(at your option) any later version.											//
//																				//
//	This program is distributed in the hope that it will be useful,				//
//	but WITHOUT ANY WARRANTY; without even the implied warranty of				//
//	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the				//
//	GNU Affero General Public License for more details.							//
//																				//
//	You should have received a copy of the GNU Affero General Public License	//
//	along with this program.  If not, see <https://www.gnu.org/licenses/>.		//
//////////////////////////////////////////////////////////////////////////////////
#include "UIElement.h"

#include "UI.h"

#include "../Audio/Audio.h"

#include <nlnx/nx.hpp>

namespace ms
{
	UIElement::UIElement(Point<int16_t> p, Point<int16_t> d, bool a) : position(p), dimension(d), active(a)
	{}

	UIElement::UIElement(Point<int16_t> p, Point<int16_t> d) : UIElement(p, d, true)
	{}

	UIElement::UIElement() : UIElement(Point<int16_t>(), Point<int16_t>())
	{}

	void UIElement::draw(float alpha) const
	{
		draw_sprites(alpha);
		draw_buttons(alpha);
	}

	void UIElement::draw_sprites(float alpha) const
	{
		for (const Sprite &sprite : sprites)
			sprite.draw(position, alpha);
	}

	void UIElement::draw_buttons(float) const
	{
		for (auto &iter : buttons)
			if (const Button *button = iter.second.get())
				button->draw(position);
	}

	void UIElement::update()
	{
		for (auto &sprite : sprites)
			sprite.update();

		for (auto &iter : buttons)
			if (Button *button = iter.second.get())
				button->update();
	}

	void UIElement::makeactive()
	{
		active = true;
	}

	void UIElement::deactivate()
	{
		active = false;
	}

	bool UIElement::is_active() const
	{
		return active;
	}

	void UIElement::toggle_active()
	{
		if (active)
			deactivate();
		else
			makeactive();
	}

	bool UIElement::is_in_range(Point<int16_t> cursorpos) const
	{
		auto bounds = Rectangle<int16_t>(position, position + dimension);

		return bounds.contains(cursorpos);
	}

	void UIElement::remove_cursor()
	{
		release_button();

		for (auto &btit : buttons)
		{
			auto button = btit.second.get();

			if (button->get_state() == Button::State::MOUSEOVER)
				button->set_state(Button::State::NORMAL);
		}

		update_help();
	}

	Cursor::State UIElement::send_cursor(bool down, Point<int16_t> pos)
	{
		Cursor::State state = send_cursor_to_buttons(down, pos);
		update_help();

		return state;
	}

	Cursor::State UIElement::send_cursor_to_buttons(bool down, Point<int16_t> pos)
	{
		if (holding)
		{
			auto btit = buttons.find(held_button);

			if (btit == buttons.end())
			{
				holding = false;
			} else if (down)
			{
				// The pressed button keeps the cursor, and looks pressed only
				// while under it (FUN_004c0ad9).
				Button *button = btit->second.get();
				Button::State state = button->get_state();

				if (button_under(btit->first, pos))
				{
					if (state == Button::State::NORMAL)
						button->set_state(Button::State::PRESSED);
				} else if (state == Button::State::PRESSED)
				{
					button->set_state(Button::State::NORMAL);
				}

				return Cursor::State::CLICKING;
			} else if (button_under(btit->first, pos) && btit->second->get_state() == Button::State::PRESSED)
			{
				// v83 clicks a button as the mouse comes back up over it
				// (FUN_004c107d).
				holding = false;

				return click_button(btit->first);
			} else
			{
				release_button();
			}
		}

		Cursor::State ret = down ? Cursor::State::CLICKING : Cursor::State::IDLE;

		for (auto &btit : buttons)
		{
			if (button_under(btit.first, pos))
			{
				// v83 presses the button the mouse goes down on whether or not
				// it was hovered, such as one that opened under a still cursor.
				if (down && !holding && btit.second->get_state() == Button::State::NORMAL)
					btit.second->set_state(Button::State::MOUSEOVER);

				if (btit.second->get_state() == Button::State::NORMAL)
				{
					// v83's tab strips (CCtrlTab, FUN_004dd822) make no sound on hover.
					if (!btit.second->is_tab())
						Sound(Sound::Name::BUTTONOVER).play();

					btit.second->set_state(Button::State::MOUSEOVER);
					ret = Cursor::State::CANCLICK;
				} else if (btit.second->get_state() == Button::State::MOUSEOVER)
				{
					if (down && !holding && btit.second->is_tab())
					{
						ret = click_button(btit.first);
					} else if (down && !holding)
					{
						// v83 only presses a button as the mouse goes down
						// (FUN_004c1059).
						holding = true;
						held_button = btit.first;

						btit.second->set_state(Button::State::PRESSED);

						ret = Cursor::State::CLICKING;
					} else if (!down)
					{
						ret = Cursor::State::CANCLICK;
					}
				}
			} else if (btit.second->get_state() == Button::State::MOUSEOVER)
			{
				btit.second->set_state(Button::State::NORMAL);
			}
		}

		return ret;
	}

	Cursor::State UIElement::click_button(uint16_t buttonid)
	{
		auto btit = buttons.find(buttonid);

		// A tab strip plays UI.img/Tab as it selects (FUN_004dd8c0).
		if (!has_own_click_sound(buttonid))
			Sound(btit != buttons.end() && btit->second->is_tab() ? Sound::Name::TAB : Sound::Name::BUTTONCLICK).play();

		Button::State state = button_pressed(buttonid);
		btit = buttons.find(buttonid);

		if (btit == buttons.end())
			return Cursor::State::IDLE;

		// v83 puts a clicked button back under the cursor, so the next click
		// needs no fresh hover (FUN_004c107d).
		if (state == Button::State::NORMAL && active && btit->second->is_active())
			state = Button::State::MOUSEOVER;

		btit->second->set_state(state);

		return state == Button::State::MOUSEOVER ? Cursor::State::CANCLICK : Cursor::State::IDLE;
	}

	bool UIElement::button_under(uint16_t buttonid, Point<int16_t> pos) const
	{
		auto btit = buttons.find(buttonid);

		return btit != buttons.end() && btit->second->is_active() && btit->second->bounds(position).contains(pos)
			&& !button_covered(buttonid, pos);
	}

	void UIElement::set_button_help(uint16_t buttonid, const std::string &path, bool above)
	{
		nl::node src = nl::nx::string["ToolTipHelp.img"].resolve(path);

		if (!src)
			return;

		button_help[buttonid] = { src["Title"].get_string(), src["Desc"].get_string(), above };
	}

	// A disabled or checked button never looks hovered, and so shows none
	// (FUN_004c0ad9).
	void UIElement::update_help()
	{
		for (auto &[id, help] : button_help)
		{
			auto btit = buttons.find(id);

			if (active && btit != buttons.end() && btit->second->is_active()
				&& btit->second->get_state() == Button::State::MOUSEOVER)
			{
				Rectangle<int16_t> bounds = btit->second->bounds(position);
				UI::get().show_help(get_type(), help.title, help.desc, Point<int16_t>(bounds.right(), bounds.top()),
									help.above);
				help_up = true;

				return;
			}
		}

		if (help_up)
		{
			help_up = false;
			UI::get().clear_help(get_type());
		}
	}

	void UIElement::release_button()
	{
		if (!holding)
			return;

		holding = false;

		auto btit = buttons.find(held_button);

		if (btit != buttons.end() && btit->second->get_state() == Button::State::PRESSED)
			btit->second->set_state(Button::State::NORMAL);
	}
}