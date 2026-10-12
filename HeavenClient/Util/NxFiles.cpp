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
#include "NxFiles.h"

#include <fstream>
#include <memory>

#include <nlnx/file.hpp>
#include <nlnx/nx.hpp>

namespace ms
{
	namespace NxFiles
	{
		nl::node ui_classic;

		Error init()
		{
			for (auto filename : filenames)
				if (std::ifstream{ filename }.good() == false)
					return Error(Error::Code::MISSING_FILE, filename);

			try
			{
				nl::nx::load_all();

				// nl::nx::load_all only knows the standard files, so this one is
				// opened here and stays open for the life of the client.
				static std::unique_ptr<nl::file> ui_classic_file = std::make_unique<nl::file>("UIClassic.nx");
				ui_classic = *ui_classic_file;
			}
			catch (const std::exception& ex)
			{
				static const std::string message = ex.what();

				return Error(Error::Code::NLNX, message.c_str());
			}

			constexpr const char* POSTCHAOS_BITMAP = "Login.img/WorldSelect/BtChannel/layer:bg";

			if (nl::nx::ui.resolve(POSTCHAOS_BITMAP).data_type() != nl::node::type::bitmap)
				return Error::Code::WRONG_UI_FILE;

			// The pre-Big-Bang minimap rules out the newer UI.wz, and the Aran
			// skill guide rules out UI.wz files from before v83.
			constexpr const char* PREBB_BITMAP = "UIWindow.img/MiniMap/title";
			constexpr const char* ARAN_BITMAP = "UIWindow.img/AranSkillGuide/0";

			if (ui_classic.resolve(PREBB_BITMAP).data_type() != nl::node::type::bitmap
				|| ui_classic.resolve(ARAN_BITMAP).data_type() != nl::node::type::bitmap)
				return Error::Code::WRONG_UI_CLASSIC_FILE;

			return Error::Code::NONE;
		}
	}
}