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
#include "UIChannel.h"

#include "../KeyAction.h"
#include "../UI.h"
#include "UIWorldSelect.h"

#include "../Components/MapleButton.h"
#include "../Components/AreaButton.h"
#include "../Audio/Audio.h"

#include "../../Configuration.h"
#include "../../Net/Packets/GameplayPackets.h"
#include "../../Util/NxFiles.h"

#include <algorithm>
#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// The window, measured from period footage. The top piece ends where
		// the channel grid starts; the middle piece is tiled down to the bottom
		// piece, which follows the last row.
		constexpr Point<int16_t> WORLD = Point<int16_t>(16, 30);
		constexpr int16_t GRID_TOP = 67;
		constexpr Point<int16_t> CELL = Point<int16_t>(9, GRID_TOP);
		constexpr Point<int16_t> CELL_PITCH = Point<int16_t>(70, 21);
		constexpr Point<int16_t> LABEL = Point<int16_t>(8, 5);
		constexpr int16_t COLUMNS = 6;
		constexpr Point<int16_t> BT_CHANGE = Point<int16_t>(286, 11);
		constexpr Point<int16_t> BT_CANCEL = Point<int16_t>(380, 11);
	}

	UIChannel::UIChannel() : UIDragElement<PosCHANNEL>()
	{
		uint8_t selected_world_id = Configuration::get().get_worldid();
		current_channel = Configuration::get().get_channelid();
		channel_count = 1;

		if (auto worldselect = UI::get().get_element<UIWorldSelect>())
			channel_count = worldselect->get_channel_count(selected_world_id);
		else
			channel_count = Configuration::get().get_world_channel_count(selected_world_id);

		nl::node Channel = NxFiles::ui_classic["UIWindow.img"]["Channel"];
		nl::node labels = Channel["ch"];

		channel_count = std::max<uint8_t>(1, std::min<uint8_t>(channel_count, labels.size()));

		if (current_channel >= channel_count)
			current_channel = 0;

		selected_channel = current_channel;

		Texture top = Channel["t"];
		Texture middle = Channel["c"];
		Texture bottom = Channel["s"];

		int16_t rows = (channel_count + COLUMNS - 1) / COLUMNS;
		int16_t bottom_y = GRID_TOP + CELL_PITCH.y() * rows;

		sprites.emplace_back(Channel["t"]);

		// The last tile overlaps the one before it rather than run past the bottom.
		for (int16_t y = top.height(); y < bottom_y; y += middle.height())
			sprites.emplace_back(Channel["c"], Point<int16_t>(0, std::min<int16_t>(y, bottom_y - middle.height())));

		sprites.emplace_back(Channel["s"], Point<int16_t>(0, bottom_y));
		sprites.emplace_back(Channel["world"][std::to_string(selected_world_id)], WORLD);

		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(Channel["BtCancel"], BT_CANCEL + Point<int16_t>(0, bottom_y));
		buttons[Buttons::CHANGE] = std::make_unique<MapleButton>(Channel["BtChange"], BT_CHANGE + Point<int16_t>(0, bottom_y));

		cell[Cell::EMPTY] = Channel["channel0"];
		cell[Cell::CURRENT] = Channel["channel1"];
		cell[Cell::SELECTED] = Channel["channel2"];
		cells = rows * COLUMNS;

		for (uint8_t i = 0; i < channel_count; i++)
		{
			ch.emplace_back(labels[i]);
			buttons[Buttons::CH + i] = std::make_unique<AreaButton>(cell_position(i), cell[Cell::EMPTY].get_dimensions());
		}

		dimension = Point<int16_t>(top.width(), bottom_y + bottom.height());
		dragarea = Point<int16_t>(dimension.x(), 20);
	}

	void UIChannel::draw(float inter) const
	{
		UIElement::draw_sprites(inter);

		for (uint8_t i = 0; i < cells; i++)
		{
			Cell kind = Cell::EMPTY;

			if (i == selected_channel)
				kind = Cell::SELECTED;
			else if (i == current_channel)
				kind = Cell::CURRENT;

			cell[kind].draw(position + cell_position(i));
		}

		for (uint8_t i = 0; i < ch.size(); i++)
			ch[i].draw(position + cell_position(i) + LABEL);

		UIElement::draw_buttons(inter);
	}

	void UIChannel::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		if (escape)
		{
			cancel();
		} else if (keycode == KeyAction::Id::RETURN)
		{
			change_channel();
		} else if (keycode == KeyAction::Id::UP)
		{
			select(selected_channel >= COLUMNS ? selected_channel - COLUMNS : selected_channel);
		} else if (keycode == KeyAction::Id::DOWN)
		{
			select(selected_channel + COLUMNS < channel_count ? selected_channel + COLUMNS : selected_channel);
		} else if (keycode == KeyAction::Id::LEFT)
		{
			select(selected_channel > 0 ? selected_channel - 1 : channel_count - 1);
		} else if (keycode == KeyAction::Id::RIGHT)
		{
			select(selected_channel + 1 < channel_count ? selected_channel + 1 : 0);
		}
	}

	Cursor::State UIChannel::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Cursor::State dstate = UIDragElement::send_cursor(clicked, cursorpos);

		if (dragged)
			return dstate;

		Cursor::State ret = clicked ? Cursor::State::CLICKING : Cursor::State::IDLE;

		for (size_t i = 0; i < channel_count + Buttons::CH; i++)
		{
			if (buttons[i]->is_active() && buttons[i]->bounds(position).contains(cursorpos))
			{
				if (buttons[i]->get_state() == Button::State::NORMAL)
				{
					if (i < Buttons::CH)
					{
						Sound(Sound::Name::BUTTONOVER).play();

						buttons[i]->set_state(Button::State::MOUSEOVER);
						ret = Cursor::State::CANCLICK;
					} else
					{
						buttons[i]->set_state(Button::State::MOUSEOVER);
						ret = Cursor::State::IDLE;
					}
				} else if (buttons[i]->get_state() == Button::State::MOUSEOVER)
				{
					if (clicked)
					{
						if (i < Buttons::CH)
							Sound(Sound::Name::BUTTONCLICK).play();

						buttons[i]->set_state(button_pressed(i));

						ret = Cursor::State::IDLE;
					} else
					{
						if (i < Buttons::CH)
							ret = Cursor::State::CANCLICK;
						else
							ret = Cursor::State::IDLE;
					}
				}
			} else if (buttons[i]->get_state() == Button::State::MOUSEOVER)
			{
				buttons[i]->set_state(Button::State::NORMAL);
			}
		}

		return ret;
	}

	UIElement::Type UIChannel::get_type() const
	{
		return TYPE;
	}

	Button::State UIChannel::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::CANCEL:
				cancel();
				break;
			case Buttons::CHANGE:
				change_channel();
				break;
			default:
				select(buttonid - Buttons::CH);
				break;
		}

		return Button::State::NORMAL;
	}

	void UIChannel::cancel()
	{
		deactivate();

		current_channel = Configuration::get().get_channelid();

		if (current_channel >= channel_count)
			current_channel = 0;

		selected_channel = current_channel;
	}

	void UIChannel::change_channel()
	{
		if (selected_channel != current_channel)
		{
			Configuration::get().set_channelid(selected_channel);
			ChangeChannelPacket(selected_channel).dispatch();
		}

		cancel();
	}

	void UIChannel::select(uint8_t channel)
	{
		selected_channel = channel;
	}

	Point<int16_t> UIChannel::cell_position(uint8_t index) const
	{
		return CELL + Point<int16_t>(CELL_PITCH.x() * (index % COLUMNS), CELL_PITCH.y() * (index / COLUMNS));
	}
}
