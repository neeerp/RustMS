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

#include "../UIDragElement.h"

#include "../../Gameplay/Stage.h"
#include "../../Graphics/Geometry.h"
#include "../../Graphics/SpecialText.h"

#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// The v83 minimap (UIWindow.img/MiniMap): folded to its title bar, the
	// map alone, or the map under its mark, street and name.
	class UIMiniMap : public UIDragElement<PosMINIMAP>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::MINIMAP;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = true;

		UIMiniMap(const CharStats &stats);

		void draw(float alpha) const override;
		void update() override;
		void remove_cursor() override;
		Cursor::State send_cursor(bool clicked, Point<int16_t> pos) override;
		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		enum Buttons : uint16_t { BT_MIN, BT_MAX, BT_MAP };
		enum Type : int8_t { MIN, NORMAL, MAX };

		// The nine pieces of a stretchable frame.
		struct Frame
		{
			Texture nw, n, ne, w, c, e, sw, s, se;
			// v83 draws the translucent white middle over black, so the field
			// doesn't show through: it comes out grey (119, 119, 119).
			ColorBox backdrop = ColorBox(1, 1, Color::Name::BLACK, 1.0f);

			Frame() = default;
			Frame(nl::node src);

			void draw(Point<int16_t> position, int16_t width, int16_t middle) const;
			int16_t top() const;
			int16_t bottom() const;
		};

		void change_map();
		void set_type(int8_t type);
		// The player's choice, kept for the next map and session.
		void choose_type(int8_t type);
		// Where a point of the map shows in the window.
		Point<int16_t> to_window(Point<int16_t> map_position) const;
		int16_t top() const;

		static constexpr int16_t MIN_WIDTH = 128;
		static constexpr int16_t BORDER = 6;

		const CharStats &stats;
		Frame normal_frame, max_frame;
		Texture min_left, min_center, min_right, title;
		nl::node markers;
		int32_t mapid = -1;
		nl::node map;
		bool has_map = false;
		int8_t type;
		// The type the player chose, kept while towns force MAX.
		int8_t user_type;
		Texture canvas, mark;
		OutlinedText street, name;
		int16_t scale = 1;
		Point<int16_t> center_offset;
		int16_t width = MIN_WIDTH;
		// Portals to other maps, by name, where they show.
		std::vector<std::pair<std::string, Point<int16_t>>> portals;
	};
}
