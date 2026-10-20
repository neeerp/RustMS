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

#include "../Physics/PhysicsObject.h"

#include "../../Graphics/Animation.h"

#include <iostream>

namespace ms
{
	class Background
	{
	public:
		// The map's back `index`, which is its node's name.
		Background(nl::node src, int16_t index);

		void draw(double viewx, double viewy, float alpha) const;

		void update();

		int16_t get_index() const;

		// SET_BACK_EFFECT: take the alpha from where it is to `target` over `duration` ms.
		void fade(float target, int32_t duration);

	private:
		enum Type
		{
			NORMAL,
			HTILED,
			VTILED,
			TILED,
			HMOVEA,
			VMOVEA,
			HMOVEB,
			VMOVEB
		};

		static Type typebyid(int32_t id)
		{
			if (id >= NORMAL && id <= VMOVEB)
				return static_cast<Type>(id);

			std::cout << "Unknown Background::Type id: [" << id << "]" << std::endl;

			return NORMAL;
		}

		void settype(Type type);

		int16_t VWIDTH;
		int16_t VHEIGHT;
		int16_t WOFFSET;
		int16_t HOFFSET;

		Animation animation;
		bool animated;
		int16_t cx;
		int16_t cy;
		double rx;
		double ry;
		int16_t htile;
		int16_t vtile;
		float opacity;
		bool flipped;
		int16_t index;

		float fade_from = 0.0f;
		float fade_to = 0.0f;
		int32_t fade_elapsed = 0;
		int32_t fade_duration = 0;

		MovingObject moveobj;
	};

	class MapBackgrounds
	{
	public:
		MapBackgrounds(nl::node src);

		MapBackgrounds();

		void drawbackgrounds(double viewx, double viewy, float alpha) const;

		void drawforegrounds(double viewx, double viewy, float alpha) const;

		void update();

		// SET_BACK_EFFECT: fade the back with this index.
		void fade(int16_t index, float target, int32_t duration);

		// CLEAR_BACK_EFFECT: load the backs again as the map has them (FUN_00644491).
		void reset();

	private:
		nl::node src;
		std::vector<Background> backgrounds;
		std::vector<Background> foregrounds;
		bool black;
	};
}