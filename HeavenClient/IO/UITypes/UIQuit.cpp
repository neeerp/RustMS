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
#include "UIQuit.h"

#include "Timer.h"
#include "UINotice.h"
#include "../PartySearchActions.h"
#include "../UI.h"
#include "../Window.h"

#include "../../Gameplay/Stage.h"
#include "../../Net/Session.h"

namespace ms
{
	namespace
	{
		void quit_to_login()
		{
			Constants::Constants::get().set_viewwidth(800);
			Constants::Constants::get().set_viewheight(600);

			Window::get().fadeout(
					[]()
					{
						GraphicsGL::get().clear();

						PartySearchActions::reset();
						UI::get().change_state(UI::State::LOGIN);
						UI::get().set_scrollnotice("");
						Session::get().reconnect();

						UI::get().enable();
						Timer::get().start();
						GraphicsGL::get().unlock();
					}
			);

			GraphicsGL::get().lock();
			Stage::get().clear();
			Timer::get().start();
		}
	}

	void ask_to_quit()
	{
		UI::get().emplace<UIYesNo>(
				"Are you sure you want to quit?",
				[](bool yes)
				{
					if (yes)
						quit_to_login();
				}
		);
	}
}
