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
#pragma once

#include <cstdint>

#include "../UIElement.h"

#include "../Components/ChatBalloon.h"

#include "../../Graphics/Animation.h"
#include "../../Net/Login.h"

namespace ms
{
	// The v83 world select, step 1 of the login scene. The worlds hang as
	// planks from the rope bar; picking one unrolls the scroll below it to
	// show that world's channels.
	class UIWorldSelect : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::WORLDSELECT;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIWorldSelect();

		void draw(float alpha) const override;
		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		void draw_world();

		void add_world(World world);

		void add_recommended_world(RecommendedWorld world);

		// Put the key focus on the world the account last played in.
		void focus_world(int32_t worldid);

		void remove_selected();

		// The v83 art for a world is keyed by the world id itself.
		std::string get_world_name(uint8_t worldid) const;
		uint8_t get_channel_count(uint8_t worldid) const;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void select_world(uint8_t index);
		void select_channel(uint8_t channel);
		void close_scroll();
		void enter_world();

		int16_t plank_at(Point<int16_t> cursorpos) const;
		int16_t channel_at(Point<int16_t> cursorpos) const;

		enum Buttons : uint16_t
		{
			BT_ENTERWORLD,
			BT_VIEWRECOMMENDED,
			BT_VIEWALLCHAR,
			BT_BACK
		};

		enum class Scroll : uint8_t
		{
			CLOSED,
			OPENING,
			OPEN,
			CLOSING
		};

		struct Plank
		{
			Texture normal;
			Animation mouseover;
			Texture mouseover_end;
			Texture pressed;
			Texture focused;
			bool hovered = false;
			bool hover_done = false;
		};

		std::vector<World> worlds;
		std::vector<RecommendedWorld> recommended_worlds;
		std::vector<Plank> planks;
		Texture empty_plank;

		Scroll scroll = Scroll::CLOSED;
		Animation scroll_open;
		Animation scroll_close;
		Texture scroll_opened;
		Texture channel_background;
		Texture world_name;
		std::vector<Texture> channels;
		Texture gauge;
		Animation channel_select;
		Texture channel_selected;
		bool channel_select_done = false;

		ChatBalloonHorizontal chatballoon;

		// Slots into `worlds`; -1 when none.
		int16_t selected = -1;
		int16_t focused = -1;
		uint8_t channelid = 0;
	};
}
