#include "pch.h"
#include <iostream>
#include <fstream>
#include <map>
#include <vector>
#include <filesystem>
#include <kaiju_compiler_capi.hpp>
#include <kaiju_vm_capi.hpp>

class FileManager {
public:
	bool load(const char* path, const std::string name) {
		if (m_files.count(name)) {
			return true;
		}
		std::ifstream file(path, std::ios::binary | std::ios::ate);
		if (!file) {
			return false;
		}
		auto size = (unsigned int)file.tellg();
		auto bytes = std::vector<unsigned char>(size);
		file.seekg(0, std::ios::beg);
		file.read((char*)bytes.data(), size);
		file.close();
		m_files.emplace(std::move(name), bytes);
		return true;
	}

	const std::vector<unsigned char>* get(const std::string& name) const {
		if (m_files.count(name)) {
			return &m_files.at(name);
		}
		return nullptr;
	}

private:
	std::map<std::string, std::vector<unsigned char>> m_files;
};

const unsigned char* onServeFile(void* context, const char* path, uintptr_t* outSize) {
	auto data = ((FileManager*)context)->get(path);
	if (data == nullptr) {
		*outSize = 0;
		return nullptr;
	}
	*outSize = data->size();
	return data->data();
}

void onResultPstFile(void* context, const unsigned char* bytes, uintptr_t size) {
	if (bytes != nullptr && size > 0) {
		((std::string*)context)->assign((const char*)bytes, size);
	}
}

void onResultBinFile(void* context, const unsigned char* bytes, uintptr_t size) {
	if (bytes != nullptr && size > 0) {
		auto ctx = (std::vector<unsigned char>*)context;
		ctx->resize(size);
		std::memcpy(ctx->data(), bytes, size);
	}
}

void onError(void* context, const char* error) {
	std::cerr << error << std::endl;
}

template<typename T>
const T* getValue(uintptr_t address) {
	if (address + sizeof(T) <= kaiju_state_size()) {
		return (const T*)kaiju_state_ptr(address);
	}
	return nullptr;
}

template<typename T>
void setValue(uintptr_t address, const T& value) {
	if (address + sizeof(T) <= kaiju_state_size()) {
		*(T*)kaiju_state_ptr_mut(address) = value;
	}
}

void onProcessOp(void* context, const char* op, const uintptr_t* params, uintptr_t params_count, const uintptr_t* targets, uintptr_t targets_count) {
	auto id = std::string(op);
	if (id == "add") {
		auto a = *getValue<int32_t>(params[0]);
		auto b = *getValue<int32_t>(params[1]);
		auto c = a + b;
		setValue<int32_t>(targets[0], c);
	}
	else if (id == "out") {
		auto v = *getValue<int32_t>(params[0]);
		std::cout << "out: " << v << std::endl;
	}
}

int main()
{
	auto context = FileManager();
	context.load("res/program.kj", "program.kj");
	context.load("res/descriptor.kjo", "descriptor.kjo");
	std::string resultPst;
	std::vector<unsigned char> resultBin;

	kaiju_compile_program_pst(
		"program.kj",
		"descriptor.kjo",
		true,
		onServeFile,
		&context,
		onResultPstFile,
		&resultPst,
		onError,
		nullptr
	);
	std::cout << resultPst << std::endl;

	kaiju_compile_program_bin(
		"program.kj",
		"descriptor.kjo",
		onServeFile,
		&context,
		onResultBinFile,
		&resultBin,
		onError,
		nullptr
	);

	kaiju_run_program(
		resultBin.data(),
		resultBin.size(),
		"main",
		1024,
		1024,
		onProcessOp,
		nullptr,
		onError,
		nullptr
	);
}
