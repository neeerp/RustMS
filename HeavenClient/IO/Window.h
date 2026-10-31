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

#include "../Error.h"

#include "../Template/Singleton.h"

//#include <GL/glew.h>
#include <glad/glad.h>
#include <GLFW/glfw3.h>
#include <string>
#include <functional>
#include <optional>

#ifdef WINDOWS
#include <direct.h>
#define GetCurrentDir _getcwd
#else

#include <unistd.h>
//#include <GL/gl.h>
#define GetCurrentDir getcwd
#endif

#include<iostream>

namespace ms
{
	class Window : public Singleton<Window>
	{
	public:
		Window();

		~Window();

		Error init();

		Error initwindow();

		bool not_closed() const;

		void update();

		void begin() const;

		void end() const;

		// Fade the screen to black, run fadeprocedure, then fade back in.
		void fadeout(std::function<void()> fadeprocedure);

		void check_events();

		void setclipboard(const std::string &text) const;

		std::string getclipboard() const;

		// Puts the mouse pointer at `x, y` in the window, as if moved there.
		void move_cursor(int16_t x, int16_t y);

		void toggle_fullscreen();

		// Save the next frame as the next free MapleNNNN.jpg in the working
		// directory.
		void screenshot();
		// Save the next frame as `name`_YYYYMMDD_HHMMSS.jpg, a Lie
		// Detector's evidence.
		void screenshot(const std::string &name);

	private:
		void updateopc();

		GLFWwindow *glwnd;
		GLFWwindow *context;
		bool fullscreen;
		float opacity;
		float opcstep;
		std::function<void()> fadeprocedure;
		int16_t width;
		int16_t height;
		mutable bool screenshot_pending = false;
		mutable std::optional<std::string> screenshot_name;

		void save_screenshot() const;

		std::string GetCurrentWorkingDir(void) const;
	};
}