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
#include "Window.h"

#include "UI.h"

#include "../Configuration.h"
#include "../Constants.h"
#include "../Timer.h"
#include "ScreenshotRules.h"
#include "UiControl.h"

#include <cmath>
#include <ctime>
#include <fstream>
#include <vector>

#define STB_IMAGE_IMPLEMENTATION
// TODO: (rich) is needed?
//#include <stb_image.h>
#ifdef WIN32
#include <Windows.h>
#else
#include "stb_image.h"
#endif

#define STB_IMAGE_WRITE_IMPLEMENTATION
#include "stb_image_write.h"

namespace ms
{
	Window::Window()
	{
		context = nullptr;
		glwnd = nullptr;
		opacity = 1.0f;
		opcstep = 0.0f;
		width = Constants::Constants::get().get_viewwidth();
		height = Constants::Constants::get().get_viewheight();
	}

	Window::~Window()
	{
		glfwTerminate();
	}

	void error_callback(int no, const char *description)
	{
		std::cout << "GLFW error [" << no << "]: " << description << std::endl;
	}

	void key_callback(GLFWwindow *, int key, int, int action, int)
	{
		UI::get().send_key(key, action != GLFW_RELEASE);
	}

	std::chrono::time_point<std::chrono::steady_clock> start = ContinuousTimer::get().start();
	// Where the last click came up; a double click is two in one place, as
	// Windows counts it (SM_CXDOUBLECLK, 4 pixels).
	double last_x = -100.0, last_y = -100.0;

	void mousekey_callback(GLFWwindow *window, int button, int action, int)
	{
		switch (button)
		{
			case GLFW_MOUSE_BUTTON_LEFT:
				switch (action)
				{
					case GLFW_PRESS:
						UI::get().send_cursor(true);
						break;
					case GLFW_RELEASE:
					{
						auto diff_ms = ContinuousTimer::get().stop(start) / 1000;
						start = ContinuousTimer::get().start();

						double x, y;
						glfwGetCursorPos(window, &x, &y);
						bool same_place = std::abs(x - last_x) <= 2.0 && std::abs(y - last_y) <= 2.0;
						last_x = x;
						last_y = y;

						if (diff_ms > 10 && diff_ms < 200 && same_place)
							UI::get().doubleclick();

						UI::get().send_cursor(false);
					}
						break;
				}

				break;
			case GLFW_MOUSE_BUTTON_RIGHT:
				// v83 acts on the right button coming up, in the field, the
				// buff list and the Monster Book (WM_RBUTTONUP).
				switch (action)
				{
					case GLFW_RELEASE:
						UI::get().rightclick();
						break;
				}

				break;
		}
	}

	void cursor_callback(GLFWwindow *, double xpos, double ypos)
	{
		int16_t x = static_cast<int16_t>(xpos);
		int16_t y = static_cast<int16_t>(ypos);
		Point<int16_t> pos = Point<int16_t>(x, y);
		UI::get().send_cursor(pos);
	}

	void focus_callback(GLFWwindow *, int focused)
	{
		UI::get().send_focus(focused);
	}

	void scroll_callback(GLFWwindow *, double xoffset, double yoffset)
	{
		UI::get().send_scroll(yoffset);
	}

	void close_callback(GLFWwindow *window)
	{
		UI::get().send_close();

		glfwSetWindowShouldClose(window, GL_FALSE);
	}

	Error Window::init()
	{
		fullscreen = Setting<Fullscreen>::get().load();

		if (!glfwInit())
			return Error::Code::GLFW;

		glfwWindowHint(GLFW_VISIBLE, GL_FALSE);
		context = glfwCreateWindow(1, 1, "", nullptr, nullptr);
		glfwMakeContextCurrent(context);
		glfwSetErrorCallback(error_callback);
		glfwWindowHint(GLFW_VISIBLE, GL_TRUE);
		glfwWindowHint(GLFW_RESIZABLE, GL_FALSE);

		if (Error error = GraphicsGL::get().init())
			return error;

		return initwindow();
	}

	Error Window::initwindow()
	{
		if (glwnd)
			glfwDestroyWindow(glwnd);

		glwnd = glfwCreateWindow(
				width,
				height,
				Configuration::get().get_title().c_str(),
				fullscreen ? glfwGetPrimaryMonitor() : nullptr,
				context
		);

		if (!glwnd)
			return Error::Code::WINDOW;

		glfwMakeContextCurrent(glwnd);

		bool vsync = Setting<VSync>::get().load();
		glfwSwapInterval(vsync ? 1 : 0);

		glViewport(0, 0, width, height);
		glMatrixMode(GL_PROJECTION);
		glLoadIdentity();

		glfwSetInputMode(glwnd, GLFW_CURSOR, GLFW_CURSOR_HIDDEN);

		double xpos, ypos;

		glfwGetCursorPos(glwnd, &xpos, &ypos);
		cursor_callback(glwnd, xpos, ypos);

		glfwSetInputMode(glwnd, GLFW_STICKY_KEYS, GL_TRUE);
		glfwSetKeyCallback(glwnd, key_callback);
		glfwSetMouseButtonCallback(glwnd, mousekey_callback);
		glfwSetCursorPosCallback(glwnd, cursor_callback);
		glfwSetWindowFocusCallback(glwnd, focus_callback);
		glfwSetScrollCallback(glwnd, scroll_callback);
		glfwSetWindowCloseCallback(glwnd, close_callback);

		/*char buf[256];
		GetCurrentWorkingDir()
		GetCurrentDirectoryA(256, buf);*/
		std::string icon_path = GetCurrentWorkingDir() + "/Icon.png";
		//strcat(buf, "\\Icon.png");

		GLFWimage images[1];

		auto stbi = stbi_load(icon_path.c_str(), &images[0].width, &images[0].height, 0, 4);

		if (stbi == NULL)
			return Error(Error::Code::MISSING_ICON, stbi_failure_reason());

		images[0].pixels = stbi;

		glfwSetWindowIcon(glwnd, 1, images);
		stbi_image_free(images[0].pixels);

		GraphicsGL::get().reinit();

		return Error::Code::NONE;
	}

	bool Window::not_closed() const
	{
		return glfwWindowShouldClose(glwnd) == 0;
	}

	void Window::update()
	{
		updateopc();
	}

	void Window::updateopc()
	{
		if (opcstep != 0.0f)
		{
			opacity += opcstep;

			if (opacity >= 1.0f)
			{
				opacity = 1.0f;
				opcstep = 0.0f;
			} else if (opacity <= 0.0f)
			{
				opacity = 0.0f;

				// The new stage comes back in over 600 ms, or 300 ms while
				// LOCK_UI holds the UI (FUN_00776b64).
				float fadein = ui_control::state().locked ? 300.0f : 600.0f;
				opcstep = static_cast<float>(Constants::TIMESTEP) / fadein;

				fadeprocedure();
			}
		}
	}

	void Window::check_events()
	{
		int16_t max_width = Configuration::get().get_max_width();
		int16_t max_height = Configuration::get().get_max_height();
		int16_t new_width = Constants::Constants::get().get_viewwidth();
		int16_t new_height = Constants::Constants::get().get_viewheight();

		if (width != new_width || height != new_height)
		{
			width = new_width;
			height = new_height;

			if (new_width >= max_width || new_height >= max_height)
				fullscreen = true;

			initwindow();
		}

		glfwPollEvents();
	}

	void Window::begin() const
	{
		GraphicsGL::get().clearscene();
	}

	void Window::end() const
	{
		GraphicsGL::get().flush(opacity);

		if (screenshot_pending)
			save_screenshot();

		glfwSwapBuffers(glwnd);
	}

	void Window::screenshot()
	{
		screenshot_pending = true;
		screenshot_name.reset();
	}

	void Window::screenshot(const std::string &name)
	{
		screenshot_pending = true;
		screenshot_name = name;
	}

	void Window::save_screenshot() const
	{
		screenshot_pending = false;

		// v83 asks for the first free index before every shot and writes the
		// 800x600 frame as a quality 75 JPEG (IJL jquality 0x4b, FUN_00744d77).
		std::string folder = GetCurrentWorkingDir() + "/";
		std::string file;

		if (screenshot_name)
		{
			std::time_t now = std::time(nullptr);
			file = Screenshot::named_file(*screenshot_name, *std::localtime(&now));
			screenshot_name.reset();
		}
		else
		{
			file = Screenshot::file_name(Screenshot::first_free([&](int32_t i)
			{
				return std::ifstream(folder + Screenshot::file_name(i)).good();
			}));
		}

		std::vector<uint8_t> pixels(static_cast<size_t>(width) * height * 3);
		glPixelStorei(GL_PACK_ALIGNMENT, 1);
		glReadPixels(0, 0, width, height, GL_RGB, GL_UNSIGNED_BYTE, pixels.data());

		stbi_flip_vertically_on_write(1);
		stbi_write_jpg((folder + file).c_str(), width, height, 3, pixels.data(), 75);
	}

	void Window::fadeout(std::function<void()> fadeproc)
	{
		// v83 fades a closing stage to black over 600 ms (FUN_00776e65) and
		// the next one back in over 600 ms (FUN_00776b64).
		opcstep = -static_cast<float>(Constants::TIMESTEP) / 600.0f;
		fadeprocedure = fadeproc;
	}

	void Window::cutout(std::function<void()> fadeproc)
	{
		opacity = 0.0f;
		opcstep = -1.0f;
		fadeprocedure = fadeproc;
	}

	void Window::setclipboard(const std::string &text) const
	{
		glfwSetClipboardString(glwnd, text.c_str());
	}

	void Window::move_cursor(int16_t x, int16_t y)
	{
		glfwSetCursorPos(glwnd, x, y);
		cursor_callback(glwnd, x, y);
	}

	std::string Window::getclipboard() const
	{
		const char *text = glfwGetClipboardString(glwnd);

		return text ? text : "";
	}

	void Window::toggle_fullscreen()
	{
		int16_t max_width = Configuration::get().get_max_width();
		int16_t max_height = Configuration::get().get_max_height();

		if (width < max_width && height < max_height)
		{
			fullscreen = !fullscreen;
			Setting<Fullscreen>::get().save(fullscreen);

			initwindow();
			glfwPollEvents();
		}
	}

	std::string Window::GetCurrentWorkingDir(void) const
	{
		char buff[FILENAME_MAX];
		GetCurrentDir(buff, FILENAME_MAX);
		std::string current_working_dir(buff);
		return current_working_dir;
	}
}