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

#include "DrawArgument.h"
#include "GraphicsGL.h"

namespace ms
{
	class Geometry
	{
	public:
		virtual ~Geometry()
		{}

	protected:
		void draw(int16_t x, int16_t y, int16_t w, int16_t h, Color::Name color, float opacity) const;
	};

	class ColorBox : public Geometry
	{
	public:
		ColorBox(int16_t width, int16_t height, Color::Name color, float opacity);

		ColorBox();

		void setwidth(int16_t width);

		void setheight(int16_t height);

		void set_color(Color::Name color);

		void setopacity(float opacity);

		void draw(const DrawArgument &args) const;

	private:
		int16_t width;
		int16_t height;
		Color::Name color;
		float opacity;
	};

	class ColorLine : public Geometry
	{
	public:
		ColorLine(int16_t width, Color::Name color, float opacity);

		ColorLine();

		void setwidth(int16_t width);

		void set_color(Color::Name color);

		void setopacity(float opacity);

		void draw(const DrawArgument &args) const;

	private:
		int16_t width;
		Color::Name color;
		float opacity;
	};

	// The HP bar over a mob or a party member, as v83 draws it (MapleLegends
	// client: FUN_005d019c for mobs, FUN_0050c8d7 for party members): a black
	// box with a white ring, and a fill 4 px tall whose bottom row is dark red.
	class MobHpBar : public Geometry
	{
	public:
		static constexpr int16_t WIDTH = 52;
		static constexpr int16_t HEIGHT = 10;

		// Draws the bar with its top-left at `position`, filled green.
		void draw(Point<int16_t> position, int16_t hppercent) const;

		// The same bar in another color, as over party members' heads.
		void draw(Point<int16_t> position, int16_t hppercent, Color::Name fill) const;

	private:
		// The fill's width at full HP.
		static constexpr int16_t FILL = 46;
	};
}